#!/usr/bin/env nu

let dir = $env.FILE_PWD
let self = [ $dir "raspberryPi4.nu" ] | path join
let root = $dir | path dirname | path dirname
let artifacts = [ $root "artifacts" ] | path join
let pidgeons = [ $root "assets" "pidgeon" "pidgeons.json" ] | path join
let flake = $"git+file:($root)"
let system = "aarch64-linux"
let format = "sd-aarch64"

def "main" [] {
  nu $self --help
}

def "main vpn" [] {
  let host = open --raw /etc/hostname | str trim

  let config = vault kv get -format=json "kv/ozds/vpn"
    | from json
    | get data.data
    | get $host
}

def "main secrets" [id?: string] {
  let pidgeon = (pick pidgeon $id)

  rm -rf $artifacts
  mkdir $artifacts
  cd $artifacts

  $pidgeon.spec | rumor stdin json --stay
}

def "main image" [id?: string] {
  let pidgeon = (pick pidgeon $id)

  rm -rf $artifacts
  mkdir $artifacts
  cd $artifacts

  let raw = (nixos-generate
    --system $system
    --format $format
    --flake $pidgeon.configuration)

  let compressed = ls ($raw
    | path dirname --num-levels 2
    | path join "sd-image")
    | get name
    | first
  unzstd $compressed -o image.img
  chmod 644 image.img

  let age = $pidgeon.secrets."scrt.key"
    | str replace "\\" "\\\\"
    | str replace "\n" "\\n"
    | str replace "\"" "\\\""

  let commands = $"run
mount /dev/sda2 /
mkdir-p /root
chmod 700 /root
write /root/.sops.age \"($age)\"
chmod 400 /root/.sops.age
exit"

  echo $commands | guestfish --rw -a image.img
}

def "main ssh" [id?: string] {
  let pidgeon = (pick pidgeon $id)

  ssh-agent bash -c $"echo '($pidgeon.secrets."ssh.key")' \\
    | ssh-add - \\
    && ssh altibiz@($pidgeon.ip)"
}

def "main pass" [id?: string] {
  let pidgeon = (pick pidgeon $id)
  $pidgeon.user-pass-priv
}

def "main deploy" [id?: string] {
  let pidgeon = (pick pidgeon $id)
  ssh-agent bash -c $"echo '($pidgeon.secrets."ssh.key")' \\
    | ssh-add - \\
    && export SSHPASS='($pidgeon.secrets."pass")' \\
    && sshpass -e deploy \\
      --remote-build \\
      --skip-checks \\
      --interactive-sudo true \\
      --hostname ($pidgeon.ip) \\
      -- \\
      '($root)#($pidgeon.configuration)'"
}

def "main db user" [id?: string] {
  let pidgeon = (pick pidgeon $id)

  let auth = $"altibiz:($pidgeon.secrets."altibiz.db.user")"
  let conn = $"($pidgeon.ip):5433"

  usql $"postgres://($auth)@($conn)/ozds"
}

def "main db admin" [id?: string] {
  let pidgeon = (pick pidgeon $id)

  let auth = $"postgres:($pidgeon.secrets."postgres.db.user")"
  let conn = $"($pidgeon.ip):5433"

  usql $"postgres://($auth)@($conn)/postgres"
}

def "pick pidgeon" [id?: string] {
  mut id = $id

  let pidgeons = (open --raw $pidgeons) | from json

  if $id == null {
    let ids = $pidgeons | get id
    $id = (gum choose --header "Pick pidgeon id:" ...($ids))
  }

  let pidgeon = $pidgeons
    | where $it.id == $id
    | first
  let secrets = vault kv get -format=json $"kv/ozds/pidgeon/($id)/current"
    | from json
    | get data.data
  let configuration = $"pidgeon-($id)-raspberryPi4-($system)"
  let expr = $"\(builtins.getFlake \"($flake)\"\).lib.rumor.\"($configuration)\""
  let spec = nix eval --json --impure --expr $expr
  $pidgeon
    | insert secrets $secrets
    | insert configuration $configuration
    | insert spec $spec
}
