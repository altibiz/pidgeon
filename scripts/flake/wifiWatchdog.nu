#!/usr/bin/env nu

const STATE_DIR = "/var/lib/wifi-watchdog"
const STATE_FILE = ($STATE_DIR | path join "state.json")
const REBOOT_FILE = ($STATE_DIR | path join "reboot.json")
const RESETS_LOG = ($STATE_DIR | path join "resets.log")
const HISTORY = ($STATE_DIR | path join "history.log")
const INCIDENTS = ($STATE_DIR | path join "incidents")
const RELOAD_UNIT = "wifi-watchdog-reload"

const KEEP_INCIDENTS = 20
const HISTORY_MAX = 1mb
const HISTORY_KEEP_LINES = 5000

const CONFIRM_FAILS = 2
const RUNGS = ["none" "reconnect" "reload" "reboot"]
const SETTLE_S = [0 120 240 0]
const RELOAD_RETRY_S = 900
const LINK_REBOOT_AFTER_S = 21600
const REBOOT_COOLDOWNS_S = [3600 3600 7200 14400 28800]
const REBOOT_STALL_S = 600

const WEDGE = ["missing" "driver_hung" "driver_timeout" "driver_error" "stuck_process"]
const STUCK_COMMS = ["iw" "modprobe" "rmmod" "nmcli" "wpa_supplicant" "NetworkManager"]
const REBOOT_FRESH = {count: 0, streak: 0, last_epoch: 0, last_iso: "", last_status: ""}

def iface [] {
  $env.WIFI_WATCHDOG_INTERFACE? | default "wlan0"
}

def now-epoch [] {
  date now | format date "%s" | into int
}

def now-iso [] {
  date now | date to-timezone UTC | format date "%Y-%m-%dT%H:%M:%SZ"
}

def uptime-s [] {
  ^cat /proc/uptime | split row " " | first | into float | into int
}

def boot-id [] {
  ^cat /proc/sys/kernel/random/boot_id | str trim
}

def path-env [] {
  let p = $env.PATH
  if ($p | describe | str starts-with "list") { $p | str join ":" } else { $p }
}

def emit [level: string, event: string, msg: string, extra: record = {}] {
  let kv = ($extra | transpose k v | each {|r| $"($r.k)=($r.v)" } | str join " ")
  let clean = ($msg | str replace --all --regex '\s+' ' ' | str replace --all '"' "'" | str trim)
  let line = $"(now-iso) ($level) iface=(iface) event=($event) ($kv) msg=\"($clean)\""
  let prio = ({ERROR: 3, WARN: 4, INFO: 6} | get $level)
  if ($env.INVOCATION_ID? | is-not-empty) {
    print --stderr $"<($prio)>($line)"
  } else {
    print --stderr $line
  }
  $"($line)\n" | save --append $HISTORY
}

def trim-history [] {
  if not ($HISTORY | path exists) { return }
  if (ls $HISTORY | get 0.size) < $HISTORY_MAX { return }
  let keep = (open $HISTORY | lines | last $HISTORY_KEEP_LINES | str join "\n")
  $"($keep)\n" | save --force $HISTORY
}

def load [path: string, fallback: record] {
  let data = (try { open $path } catch { null })
  if ($data | describe | str starts-with "record") { $fallback | merge $data } else { $fallback }
}

def store [path: string, data: record] {
  let tmp = $"($path).tmp"
  $data | to json | save --force $tmp
  mv --force $tmp $path
}

def fresh-state [boot: string] {
  {
    boot: $boot
    fails: 0
    first_fail_epoch: 0
    first_fail_up: 0
    rung: 0
    last_action_up: 0
    last_reload_up: -1
    reboot_requested_up: -1
  }
}

def stuck-procs [] {
  ^ps -eo "stat=,comm="
  | lines
  | parse --regex '^\s*(?P<stat>\S+)\s+(?P<comm>.+)$'
  | where ($it.stat | str starts-with "D") and ($it.comm in $STUCK_COMMS)
  | each {|p| $p.comm }
  | uniq
}

def probe-driver [] {
  let r = (^timeout --kill-after=2 8 iw dev (iface) station dump | complete)
  let err = ($r.stderr | str trim)
  if ($r.exit_code in [124 137]) {
    {ok: false, status: "driver_hung", detail: "iw station dump did not return within 8s"}
  } else if ($err =~ '-110|timed out') {
    {ok: false, status: "driver_timeout", detail: $err}
  } else if $r.exit_code != 0 {
    {ok: false, status: "driver_error", detail: $err}
  } else if ($r.stdout | str trim | is-empty) {
    {ok: false, status: "unassociated", detail: "station dump is empty"}
  } else {
    {ok: true, status: "associated", detail: ""}
  }
}

def probe-path [] {
  let gw = (try { ^ip -j route show default dev (iface) | from json | get 0.gateway } catch { "" })
  if ($gw | is-empty) {
    return {ok: false, status: "no_gateway", detail: "no default route via interface"}
  }
  if (^ping -n -q -c 2 -W 2 -I (iface) $gw | complete | get exit_code) == 0 {
    return {ok: true, status: "healthy", detail: $gw}
  }
  if (^arping -q -c 2 -w 3 -I (iface) $gw | complete | get exit_code) == 0 {
    return {ok: true, status: "healthy", detail: $"($gw) answers arp only"}
  }
  {ok: false, status: "no_data_path", detail: $"gateway ($gw) unreachable by icmp and arp"}
}

def probe [] {
  let stuck = (stuck-procs)
  if ($stuck | is-not-empty) {
    return {ok: false, status: "stuck_process", detail: $"uninterruptible sleep: ($stuck | str join ',')"}
  }
  if not ($"/sys/class/net/(iface)" | path exists) {
    return {ok: false, status: "missing", detail: "interface absent from sysfs"}
  }
  let driver = (probe-driver)
  if not $driver.ok {
    return $driver
  }
  probe-path
}

def run-captured [cmd: string] {
  let r = (^timeout --kill-after=2 15 sh -c $cmd | complete)
  $"($r.stdout)($r.stderr)"
}

def capture-incident [rung: string, p: record, st: record] {
  let now = (now-epoch)
  let ff = (if $st.first_fail_epoch > 0 { $st.first_fail_epoch } else { $now })
  let file = ($INCIDENTS | path join $"($now)-($rung).log")
  let sections = [
    {title: "kernel around first failure", cmd: $"journalctl -k --no-pager -o short-iso-precise --since @($ff - 300) --until @($ff + 120) | tail -n 3000"}
    {title: "kernel now", cmd: "journalctl -k --no-pager -o short-iso-precise -n 300"}
    {title: "kernel signals", cmd: $"journalctl -k --no-pager -o short-iso-precise --since @($ff - 7200) | grep -iE 'brcmf|mmc|sdio|voltage|throttl|neighbour|arp_cache' | tail -n 500"}
    {title: "pidgeon around first failure", cmd: $"journalctl -u pidgeon --no-pager -o short-iso-precise --since @($ff - 900) --until @($ff + 60) | tail -n 300"}
    {title: "uninterruptible processes", cmd: "ps -eo pid,stat,etimes,wchan:32,comm | awk 'NR == 1 || $2 ~ /^D/'"}
    {title: "syn-sent sockets", cmd: "ss -Htn state syn-sent | wc -l"}
    {title: "neighbour table", cmd: "ip neigh show | wc -l; ip neigh show | awk '{print $NF}' | sort | uniq -c"}
    {title: "interface", cmd: $"ip -s link show (iface); nmcli -t -f DEVICE,TYPE,STATE,CONNECTION device status"}
    {title: "power", cmd: "vcgencmd get_throttled; vcgencmd measure_temp; vcgencmd measure_volts core"}
  ]
  $"time=(now-iso) boot=(boot-id) rung=($rung) status=($p.status) fails=($st.fails) first_fail_epoch=($ff)\ndetail=($p.detail)\n" | save --force $file
  for s in $sections {
    $"\n===== ($s.title) =====\n(run-captured $s.cmd)" | save --append $file
  }
  ls $INCIDENTS | sort-by name --reverse | skip $KEEP_INCIDENTS | each {|f| rm --force $f.name } | ignore
  emit INFO "incident-captured" "snapshot written" {rung: $rung, file: $file}
}

def reload-running [] {
  (^systemctl is-active $"($RELOAD_UNIT).service" | complete | get stdout | str trim) in ["active" "activating" "deactivating"]
}

def act-reconnect [] {
  let r = (^nmcli -w 30 device connect (iface) | complete)
  emit WARN "reconnect" $"($r.stdout)($r.stderr)" {exit: $r.exit_code}
}

def act-reload [] {
  if (reload-running) {
    emit ERROR "reload-blocked" "previous driver reload still running, sdio likely wedged"
    return
  }
  let sh = (which sh | get 0.path)
  let cmd = "modprobe -r brcmfmac_wcc; modprobe -r brcmfmac; sleep 3; modprobe brcmfmac"
  let r = (^systemd-run --no-block --collect --quiet $"--unit=($RELOAD_UNIT)" $"--setenv=PATH=(path-env)" $sh -c $cmd | complete)
  emit WARN "driver-reload" "reloading brcmfmac in detached unit" {exit: $r.exit_code}
}

def sysrq-reboot [] {
  emit ERROR "hard-reset" "forcing emergency sync, read-only remount and reboot via sysrq"
  ^sh -c "echo s > /proc/sysrq-trigger; sleep 3; echo u > /proc/sysrq-trigger; sleep 2; echo b > /proc/sysrq-trigger"
}

def act-reboot [p: record, st: record, rb: record] {
  let count = ($rb.count + 1)
  let streak = ($rb.streak + 1)
  store $REBOOT_FILE {count: $count, streak: $streak, last_epoch: (now-epoch), last_iso: (now-iso), last_status: $p.status}
  $"(now-iso) reset=($count) streak=($streak) status=($p.status) fails=($st.fails) boot=(boot-id)\n" | save --append $RESETS_LOG
  capture-incident "reboot" $p $st
  emit WARN "reboot" $p.detail {status: $p.status, resets: $count, streak: $streak}
  ^sync
  let r = (^systemctl --no-block reboot | complete)
  if $r.exit_code != 0 {
    emit ERROR "reboot-failed" $r.stderr
    sysrq-reboot
  }
}

def reboot-or-hold [p: record, st: record, up: int, wedge: bool] {
  let rb = (load $REBOOT_FILE $REBOOT_FRESH)
  let idx = ([$rb.streak (($REBOOT_COOLDOWNS_S | length) - 1)] | math min)
  let cooldown = ($REBOOT_COOLDOWNS_S | get $idx)
  let age = ((now-epoch) - $rb.last_epoch)
  let down = ($up - $st.first_fail_up)
  let eligible = ($wedge or ($down >= $LINK_REBOOT_AFTER_S))

  if $eligible and ($age >= $cooldown) {
    store $STATE_FILE ($st | merge {rung: 3, last_action_up: $up, reboot_requested_up: $up})
    act-reboot $p $st $rb
    return
  }

  let reason = (if $eligible { "cooldown" } else { "link-only failure" })
  let extra = {reason: $reason, status: $p.status, age_s: $age, cooldown_s: $cooldown, down_s: $down, streak: $rb.streak}
  if ($up - $st.last_reload_up) >= $RELOAD_RETRY_S {
    emit WARN "reboot-held" "retrying driver reload instead" $extra
    act-reload
    store $STATE_FILE ($st | merge {rung: 2, last_action_up: $up, last_reload_up: $up})
  } else {
    emit INFO "reboot-held" "waiting" $extra
    store $STATE_FILE ($st | merge {rung: 3, last_action_up: $up})
  }
}

def "main probe" [--interface (-i): string = "wlan0"] {
  $env.WIFI_WATCHDOG_INTERFACE = $interface
  probe
}

def main [--interface (-i): string = "wlan0"] {
  $env.WIFI_WATCHDOG_INTERFACE = $interface
  mkdir $INCIDENTS
  trim-history

  let boot = (boot-id)
  let up = (uptime-s)
  let fresh = (fresh-state $boot)
  let saved = (load $STATE_FILE $fresh)

  if $saved.boot != $boot {
    if $saved.fails > 0 {
      emit INFO "new-boot" "previous boot ended unhealthy" {prev_fails: $saved.fails, prev_rung: ($RUNGS | get $saved.rung)}
    }
    store $STATE_FILE $fresh
  }
  let st = (if $saved.boot == $boot { $saved } else { $fresh })

  if ($st.reboot_requested_up >= 0) and (($up - $st.reboot_requested_up) > $REBOOT_STALL_S) {
    emit ERROR "reboot-stalled" "reboot was requested but this boot is still running" {waited_s: ($up - $st.reboot_requested_up)}
    sysrq-reboot
    return
  }

  let p = (probe)

  if $p.ok {
    if $st.fails > 0 {
      emit INFO "recovered" $p.detail {after_fails: $st.fails, rung: ($RUNGS | get $st.rung), down_s: ($up - $st.first_fail_up)}
      store $STATE_FILE $fresh
    }
    let rb = (load $REBOOT_FILE $REBOOT_FRESH)
    if $rb.streak > 0 {
      store $REBOOT_FILE ($rb | merge {streak: 0})
    }
    return
  }

  let first = ($st.fails == 0)
  let st = ($st | merge {
    fails: ($st.fails + 1)
    first_fail_epoch: (if $first { now-epoch } else { $st.first_fail_epoch })
    first_fail_up: (if $first { $up } else { $st.first_fail_up })
  })
  store $STATE_FILE $st
  let wedge = ($p.status in $WEDGE)
  emit WARN "unhealthy" $p.detail {status: $p.status, fails: $st.fails, rung: ($RUNGS | get $st.rung)}

  if $st.fails < $CONFIRM_FAILS { return }
  if ($st.rung > 0) and (($up - $st.last_action_up) < ($SETTLE_S | get $st.rung)) { return }

  let next = (if $wedge and ($st.rung == 0) { 2 } else { [($st.rung + 1) 3] | math min })

  match $next {
    1 => {
      act-reconnect
      store $STATE_FILE ($st | merge {rung: 1, last_action_up: $up})
    }
    2 => {
      if $st.last_reload_up < $st.first_fail_up { capture-incident "reload" $p $st }
      act-reload
      store $STATE_FILE ($st | merge {rung: 2, last_action_up: $up, last_reload_up: $up})
    }
    _ => { reboot-or-hold $p $st $up $wedge }
  }
}
