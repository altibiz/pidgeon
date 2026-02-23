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

def "main make-vpn" [ip: string, --host: string] {
  let host = if $host == null {
      open --raw /etc/hostname
    } else {
      $host
    } | str trim

  rm -rf $artifacts
  mkdir $artifacts
  cd $artifacts

  {
    imports: [
      {
        importer: "vault-file"
        arguments: {
          path: "kv/ozds/shared"
          file: "nebula-ca-priv"
        }
      }
      {
        importer: "vault-file"
        arguments: {
          path: "kv/ozds/shared"
          file: "nebula-ca-pub"
        }
      }
      {
        importer: "vault"
        arguments: {
          path: "kv/ozds/vpn"
        }
      }
    ]
    generations: [
      {
        generator: "nebula"
        arguments: {
          ca_private: "nebula-ca-priv"
          ca_public: "nebula-ca-pub"
          name: $host
          ip: $"($ip)/16"
          private: $"($host)-nebula-priv"
          public: $"($host)-nebula-pub"
        }
      }
      {
        generator: "moustache"
        arguments: {
          name: $host
          variables: {
            CA_PUBLIC: "nebula-ca-pub"
            CERT_PRIVATE: $"($host)-nebula-priv"
            CERT_PUBLIC: $"($host)-nebula-pub"
          }
          template: "firewall:
  inbound:
    - host: any
      port: any
      proto: any
  outbound:
    - host: any
      port: any
      proto: any
handshakes:
  try_interval: 1s
listen:
  host: 0.0.0.0
  port: 0
pki:
  ca: \"{{CA_PUBLIC}}\"
  key: \"{{CERT_PRIVATE}}\"
  cert: \"{{CERT_PUBLIC}}\"
lighthouse:
  am_lighthouse: false
  hosts:
    - 10.8.0.1
relay:
  am_relay: false
  relays:
    - 10.8.0.1
  use_relays: true
static_host_map:
  10.8.0.1:
    - ozds-vpn.altibiz.com:4242
static_map:
  cadence: 5m
  lookup_timeout: 10s
tun:
  dev: nebula.ozds-vpn
  disabled: false"
        }
      }
    ]
    exports: [
      {
        exporter: "vault"
        arguments: {
          path: "kv/ozds/vpn"
        }
      }
    ]
  } | to json | rumor stdin json --stay
}

def "main vpn" [--host: string] {
  let host = if $host == null {
      open --raw /etc/hostname
    } else {
      $host
    } | str trim

  let config = vault kv get -format=json "kv/ozds/vpn/current"
    | from json
    | get data.data
    | get $host

  let file = mktemp -t
  chmod 600 $file
  $config | save -f $file

  sudo nebula -config $file

  rm -f $file
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
    --flake $"($root)#($pidgeon.configuration)")

  let compressed = ls ($raw
    | path dirname --num-levels 2
    | path join "sd-image")
    | get name
    | first
  unzstd $compressed -o image.img
  chmod 644 image.img

  let age = $pidgeon.secrets."scrt.key"
    | str replace -a "\\" "\\\\"
    | str replace -a "\n" "\\n"
    | str replace -a "\"" "\\\""

  let commands = $"run
mount /dev/sda2 /
mkdir-p /root
chmod 700 /root
write /root/host.scrt.key \"($age)\"
chmod 400 /root/host.scrt.key
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
  $pidgeon.secrets."pass"
}

def "main deploy" [id?: string] {
  let pidgeon = (pick pidgeon $id)
  ssh-agent bash -c $"echo '($pidgeon.secrets."ssh.key")' \\
    | ssh-add - \\
    && export SSHPASS='($pidgeon.secrets."pass")' \\
    && sshpass -e deploy \\
      --skip-checks \\
      --interactive-sudo true \\
      --hostname ($pidgeon.ip) \\
      -- \\
      '($root)#($pidgeon.configuration)'"
}

def "main install" [id?: string, dev?: string] {
  let pidgeon = (pick pidgeon $id)

  let device = (pick device $dev)

  if ($device | str starts-with "/dev/sd") {
    sudo mount $"($device)2" /mnt
    sudo mount $"($device)1" /mnt/firmware
  } else {
    print "Unsupported device type"
    exit 1
  }

  try {
    # NOTE: it errors out with sandbox
    # NOTE: filter-syscalls: https://github.com/NixOS/nix/issues/5258
    (nixos-install
      --option sandbox false
      --option filter-syscalls false
      --flake $"($root)#($pidgeon.configuration)")
  } catch { |err|
    printf $"Install for ($pidgeon.id) on ($device) failed: ($err)"
    sudo umount -R /mnt
    exit 1
  }

  sudo umount -R /mnt
}

def "main update" [id?: string, dev?: string] {
  let pidgeon = (pick pidgeon $id)

  let device = (pick device $dev)

  if ($device | str starts-with "/dev/sd") {
    sudo mount $"($device)2" /mnt
    sudo mount $"($device)1" /mnt/firmware
  } else {
    print "Unsupported device type"
    exit 1
  }

  sudo mkdir -p /mnt/src
  sudo mount --bind $root /mnt/src

  try {
    # NOTE: it errors out with sandbox
    # NOTE: filter-syscalls: https://github.com/NixOS/nix/issues/5258
    (sudo nixos-enter --command
      ("nixos-rebuild boot"
        + " --option sandbox false"
        + " --option filter-syscalls false"
        + $" --flake '/src#($pidgeon.configuration)'"))
  } catch { |err|
    printf $"Update for ($pidgeon.id) on ($device) failed: ($err)"
    sudo umount /mnt/src
    sudo rmdir /mnt/src
    sudo umount -R /mnt
    exit 1
  }

  sudo umount /mnt/src
  sudo rmdir /mnt/src
  sudo umount -R /mnt
}

def --wrapped "main s3" [...args] {
  let secrets = vault kv get -format=json "kv/ozds/nix/s3.lvm.altibiz.com"
    | from json
    | get data.data

  with-env {
    AWS_ACCESS_KEY_ID: ($secrets."admin-aws-access-key-id"),
    AWS_SECRET_ACCESS_KEY: ($secrets."admin-aws-secret-access-key")
  } {
    (s3cmd
      --host=s3.lvm.altibiz.com
      "--host-bucket=s3.lvm.altibiz.com/%(bucket)"
      ...($args))
  }
}

def --wrapped "main path-info" [...args] {
  let secrets = vault kv get -format=json "kv/ozds/nix/s3.lvm.altibiz.com"
    | from json
    | get data.data

  with-env {
    AWS_ACCESS_KEY_ID: ($secrets."admin-aws-access-key-id"),
    AWS_SECRET_ACCESS_KEY: ($secrets."admin-aws-secret-access-key")
  } {
    (nix path-info
      --store s3://nix-binary-cache?endpoint=s3.lvm.altibiz.com
      ...($args))
  }
}

def "main cache" [] {
  let derivations = (open --raw $pidgeons)
    | from json
    | each { |pidgeon|
        let configuration = $"pidgeon-($pidgeon.id)-raspberryPi4-($system)"
        let expr = $"nixosConfigurations.($configuration).config.system.build.toplevel"
        $"($root)#($expr)"
      }
    | append $"($root)#packages.($system).pidgeonProbe"
    | append $"($root)#packages.($system).pidgeonCli"

  let secrets = vault kv get -format=json "kv/ozds/nix/s3.lvm.altibiz.com"
    | from json
    | get data.data

  let file = mktemp -t
  chmod 600 $file
  $secrets."private.pem" | save -f $file

  with-env {
    AWS_ACCESS_KEY_ID: ($secrets."aws-access-key-id"),
    AWS_SECRET_ACCESS_KEY: ($secrets."aws-secret-access-key")
  } {
    let cache = $"s3://nix-binary-cache?endpoint=s3.lvm.altibiz.com&secret-key=($file)"
    nix copy --to $cache ...($derivations)
  }

  rm -f $file
}

def "main db user" [id?: string] {
  let pidgeon = (pick pidgeon $id)

  let auth = $"altibiz:($pidgeon.secrets."altibiz.db.user")"
  let conn = $"($pidgeon.ip):5433"

  usql $"postgres://($auth)@($conn)/pidgeon"
}

def "main db admin" [id?: string] {
  let pidgeon = (pick pidgeon $id)

  let auth = $"postgres:($pidgeon.secrets."postgres.db.user")"
  let conn = $"($pidgeon.ip):5433"

  usql $"postgres://($auth)@($conn)/pidgeon"
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

def "pick device" [dev?: string] {
  if ($dev | is-not-empty) {
    return $dev
  }

  let dev = lsblk -o NAME -r -n -d
    | gum choose --header "Pick target device:"

  $"/dev/($dev)"
}
