#!/usr/bin/env nu

use ./static.nu *
let root = $env.FILE_PWD | path dirname
let hosts_dir = [ $root "src" "flake" "host" ] | path join

# host configuration, secret, image generation script
#
# additionally writes host images to specified locaitons
#
# start with `host create` if you're starting from scratch
# when adding new secrets for all hosts use `host generate`
#
# NOTE: any generated secret will not trump
# a previously generated secret
def "main" [ ] { }

# create host configuration, secrets and image
#
# additionally write the image to the specified destination
#
# optionally borrow wifi secrets from another host
def "main create" [
  # directory in which to generate secrets
  secrets_dir: string,
  # directory in which to generate image
  images_dir: string,
  # destination to write the image to
  destination: string,
  # id of host to borrow wifi secrets from
  --wifi-from: string,
  # set the id of the host instead of randomly generating it
  --id: string
] {
  let pwd = pwd

  let id = main init --id $id

  cd $secrets_dir
  let secrets = main secrets generate $id --wifi-from $wifi_from

  cd $images_dir
  let image = main image generate $id

  cd $pwd
  main image inject $secrets $image

  main image write $image $destination
}

# regenerate host secrets and image
#
# additionally write the image to the specified destination
def "main generate" [
  # id of host to generate secrets and image for
  id: string,
  # directory in which to generate secrets
  secrets_dir: string,
  # directory in which to generate image
  images_dir: string,
  # destination to write the image to
  destination: string,
  # id of host to borrow wifi secrets from
  --wifi-from: string
] {
  let pwd = pwd

  cd $secrets_dir
  let secrets = main secrets generate $id --wifi-from $wifi_from

  cd $images_dir
  let image = main image generate $id

  cd $pwd
  main image inject $secrets $image

  main image write $image $destination
}

# initialize host with empty configuration
# and an available ip address
def "main init" [--id: string] {
  let $hosts = static hosts $hosts_dir

  mut id = $id
  if ($id | is-empty) {
    $id = random chars --length 32
  }
  let host_dir = $"($hosts_dir)/pidgeon-($id)"
  mkdir $host_dir

  let last_ip = if (($hosts | values | length) == 0) {
      null
    } else {
      $hosts
        | get vpn.ip
        | each { |x| 
            let p = $x
              | parse "{3}.{2}.{1}.{0}"
              | each { |x|
                  {
                    0: ($x.0 | into int),
                    1: ($x.1 | into int),
                    2: ($x.2 | into int),
                    3: ($x.3 | into int)
                   }
                }
              | first
            {
              parsed: $p
              sum: ($p.0 * 2 ** 0
                + $p.1 * 2 ** 1
                + $p.2 * 2 ** 2
                + $p.3 * 2 ** 3)
              ip: $x
            }
          }
        | sort-by sum
        | last
    }

  let next_ip = if ($last_ip == null) {
    "10.8.0.10"
  } else if ($last_ip.parsed.0 == 254) {
    $"($last_ip.parsed.3).($last_ip.parsed.2).($last_ip.parsed.1 + 1).(0)"
  } else {
    $"($last_ip.parsed.3).($last_ip.parsed.2).($last_ip.parsed.1).($last_ip.parsed.0 + 1)"
  }

  echo '{ }' | try { save $"($host_dir)/config.nix" }
  {
    vpn: {
      ip: $next_ip,
      subnet: {
        ip: "10.8.0.0",
        bits: 16,
        mask: "255.255.255.0"
      }
    }
  } | to json | try { save $"($host_dir)/static.json" }

  $id
}

# generate secrets for a specified host
def "main secrets generate" [id: string, --wifi-from: string] {
  let host_dir = $"($hosts_dir)/pidgeon-($id)"
  mkdir $host_dir
  let secrets_dir = $"($id)"
  mkdir $secrets_dir

  main secrets db ca shared
  main secrets vpn ca shared

  cd $secrets_dir

  main secrets pass $id
  main secrets ssh key $id
  main secrets key $id
  main secrets vpn key $id ../shared
  main secrets db key $id ../shared
  main secrets db sql $id
  if ($wifi_from | is-empty) {
    main secrets wifi env $id
  } else {
    glob $"../($id)/($id).wifi.*"
      | each { |x|
          let suffix = $x.name
            | path basename
            | parse "{id}.wifi.{suffix}"
            | get suffix
            | first
          try { cp $x.name $"($id).wifi.($suffix)" }
        }
  }
  main secrets pidgeon env $id
  main secrets scrt key $id

  mkdir val
  cd val

  try { cp $"($id).ssh.key.pub" altibiz.ssh.pub }
  try { cp $"($id).pass.pub" altibiz.pass.pub }
  try { cp $"($id).pidgeon.env" pidgeon.env }
  try { cp $"($id).db.key" postgres.crt }
  try { cp $"($id).db.key.pub" postgres.crt.pub }
  try { cp $"($id).db.sql" postgres.sql }
  try { cp $"($id).wifi.env" wifi.env }
  try { cp ../shared.vpn.key.pub nebula.ca.pub }
  try { cp $"($id).vpn.key" nebula.crt }
  try { cp $"($id).vpn.key.pub" nebula.crt.pub }

  main secrets scrt val $id

  cd ../
  cd ../

  try { cp $"($secrets_dir)/($id).scrt.key.pub" . }
  try { cp $"($secrets_dir)/($id).scrt.key" . }
  cp -f $"($secrets_dir)/vals/($id).scrt.val.pub" .
  cp -f $"($secrets_dir)/vals/($id).scrt.val" .
  cp -f $"($id).scrt.val.pub" $"($host_dir)/secrets.yaml"

  print $"($id).scrt.key"
}

# generate a specified hosts' image
# outputs the path to the generated image
def "main image generate" [id: string] {
  (nixos-generate
    --system "aarch64-linux"
    --format "sd-aarch64"
    --flake $"($root)#pidgeon-($id)-aarch64-linux")
  unzstd ./result/sd-image/* $"./($id)-temp.img"
  mv -f  $"./($id)-temp.img" $"./($id).img" 
  ^rm -f result
  print $"./($id).img"
}

# inject secrets key into a host image
#
# requires root privileges
def "main image inject" [secrets_key: string, image: string] {
  let temp = mktemp -d
  let loop = losetup -f

  sudo losetup -P $loop $image
  sudo mount $"($loop)p2" $temp

  mkdir $"($temp)/root"
  chown root:root $"($temp)/root"
  chmod 700 $"($temp)/root"
  cp -f $secrets_key $"($temp)/root/secrets.age"
  chown root:root $"($temp)/root/secrets.age"
  chmod 400 $"($temp)/root/secrets.age"

  sudo umount $temp
  sudo losetup -d $loop
}

# write image to specified destination
#
# basically a sane wrapper over the `dd` and `sync` commands
def "main image write" [image: string, destination: string] {
  sudo dd $"if=($image)" $"of=($destination)" bs=4M conv=sync,noerror oflag=direct
  sync
}

# create a secret key
#
# assumes sops is used
#
# outputs:
#   ./name.scrt.key.pub
#   ./name.scrt.key
def "main secrets scrt key" [name: string]: nothing -> nothing {
  age-keygen err> (std null-device) out> $"($name).scrt.key"
  chmod 600 $"($name).scrt.key"

  open --raw $"($name).scrt.key"
    | (age-keygen -y
      err> (std null-device)
      out> $"($name).scrt.key.pub")
  chmod 644 $"($name).scrt.key.pub"
}

# create secret values
#
# each file in the directory starting with prefix `name` or shared
# excluding keys or values generated by scrt
# will be a secret in the resulting file
#
# each file in the directory starting with prefix `name` or shared
# and ending with .key
# will be used to encrypt the resulting file
#
# assumes sops is used
#
# outputs:
#   ./name.scrt.val.pub
#   ./name.scrt.val
def "main secrets scrt val" [name: string]: nothing -> nothing {
  ls $env.PWD
    | where { |x| $x.type == "file" }
    | where { |x| 
        let basename = $x.name | path basename
        return (
          not ($basename | str ends-with ".scrt.val")
          and not ($basename | str ends-with ".scrt.val.pub")
          and not ($basename | str ends-with ".scrt.key")
          and not ($basename | str ends-with ".scrt.key.pub")
          and (
            ($basename | str starts-with $name)
            or ($basename | str starts-with shared)
          )
        )
      }
    | each { |x|
        let content = open --raw $x.name
          | str trim
          | str replace --all "\n" "\n  "
        return $"($x.name | path basename): |\n  ($content)" 
      }
    | str join "\n"
    | save -f $"($name).scrt.val"
  chmod 600 $"($name).scrt.val"

  let keys = ls $env.PWD
    | where { |x| $x.type == "file" }
    | where { |x| 
        let basename = $x.name | path basename
        return (
          ($basename | str ends-with ".scrt.key.pub")
          and (
            ($basename | str starts-with $name)
            or ($basename | str starts-with shared)
          )
        )
      }
    | each { |x| open --raw $x.name }
    | str join ","

  (sops encrypt $"($name).scrt.val"
    --input-type yaml
    --age $keys
    --output $"($name).scrt.val.pub"
    --output-type yaml)
  chmod 644 $"($name).scrt.val.pub"
}

# create an ssh key pair
#
# assumes that openssh is used
#
# outputs:
#   ./name.ssh.key.pub
#   ./name.ssh.key
def "main secrets ssh key" [name: string]: nothing -> nothing {
  ssh-keygen -q -a 100 -t ed25519 -N "" -C $name -f $"($name).ssh.key"
  chmod 644 $"($name).ssh.key.pub"
  chmod 600 $"($name).ssh.key"
}

# create the nebula vpn ca
#
# outputs:
#   ./name.vpn.ca.pub
#   ./name.vpn.ca
def "main secrets vpn ca" [name: string]: nothing -> nothing {
  nebula-cert ca -name $name -duration $"(365 * 24 * 100)h"

  mv $"ca.crt" $"($name).vpn.ca.pub"
  chmod 644 $"($name).vpn.ca.pub"

  mv $"ca.key" $"($name).vpn.ca"
  chmod 600 $"($name).vpn.ca"
}

# create nebula vpn keys signed by a previously generated vpn ca
#
# expects the ip to be in hosts or in the VPN_`NAME`_IP env var
#
# assumes nebula vpn is used
#
# outputs:
#   ./name.vpn.key.pub
#   ./name.vpn.key
def "main secrets vpn key" [name: string, ca: path]: nothing -> nothing {
  let $hosts = static hosts $hosts_dir

  let ip_key = $"VPN_($name | str upcase)_IP"
  let ip = $env | get $ip_key --ignore-errors
    | default (($hosts | get ([ $"pidgeon-($name)" "vpn" "ip" ] | into cell-path))
        + "/"
        + ($hosts | get ([ $"pidgeon-($name)" "vpn" "subnet" "bits" ] | into cell-path)))
  if ($ip | is-empty) {
    error make {
      msg: "expected ip provided via VPN_`NAME`_IP"
    }
  }

  (nebula-cert sign
    -ca-crt $"($ca).vpn.ca.pub"
    -ca-key $"($ca).vpn.ca"
    -name $name
    -ip $ip)

  mv $"($name).crt" $"($name).vpn.key.pub"
  chmod 644 $"($name).vpn.key.pub"

  mv $"($name).key" $"($name).vpn.key"
  chmod 600 $"($name).vpn.key"
}

# create the database ssl ca
#
# assumes that the database uses openssl keys
#
# outputs:
#   ./name.db.ca.pub
#   ./name.db.ca
def "main secrets db ca" [name: string]: nothing -> nothing {
  (openssl genpkey -algorithm ED25519
    -out $"($name).db.ca")
  chmod 600 $"($name).db.ca"

  (openssl req -x509
    -key $"($name).db.ca"
    -out $"($name).db.ca.pub"
    -subj $"/CN=($name)"
    -days 3650)
  chmod 644 $"($name).db.ca.pub"
}

# create database ssl keys
# signed by a previously generated db ca
#
# assumes that the database uses openssl keys
#
# outputs:
#   ./name.db.key.pub
#   ./name.db.key
def "main secrets db key" [name: string, ca: path]: nothing -> nothing {
  (openssl genpkey -algorithm ED25519
    -out $"($name).db.key")
  chmod 600 $"($name).db.key"

  (openssl req -new
    -key $"($name).db.key"
    -out $"($name).db.key.req"
    -subj $"/CN=($name)")
  (openssl x509 -req
    -in $"($name).db.key.req"
    -CA $"($ca).db.ca.pub"
    -CAkey $"($ca).db.ca"
    -CAcreateserial
    -out $"($name).db.key.pub"
    -days 3650) err>| ignore
  rm -f $"($name).db.key.req"
  chmod 644 $"($name).db.key.pub"
}

# create initial database sql script
#
# additionally creates passwords for postgres, pidgeon
# and altibiz users
#
# assumes that a postgresql is used
#
# outputs:
#   ./name.postgres.db.user 
#   ./name.pidgeon.db.user 
#   ./name.altibiz.db.user 
#   ./name.db.sql 
def "main secrets db sql" [name: string]: nothing -> nothing {
  # create a database user password 
  #
  # outputs:
  #   ./name.db.user
  def "db user" [name: string]: nothing -> nothing {
    let key = random chars --length 32
    $key | save -f $"($name).db.user"
    chmod 600 $"($name).db.user"
  }

  db user $"($name).postgres"
  db user $"($name).pidgeon"
  db user $"($name).altibiz"

  let sql = $"ALTER USER postgres WITH PASSWORD '(open --raw $"($name).postgres.db.user")';
CREATE USER pidgeon PASSWORD '(open --raw $"($name).pidgeon.db.user")';
CREATE USER altibiz PASSWORD '(open --raw $"($name).altibiz.db.user")';

CREATE DATABASE pidgeon;
ALTER DATABASE pidgeon OWNER TO pidgeon;

\\c pidgeon

GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO altibiz;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO altibiz;
GRANT ALL PRIVILEGES ON ALL FUNCTIONS IN SCHEMA public TO altibiz;"
  $sql | try { save $"($name).db.sql" }
  chmod 600 $"($name).db.sql"
}

# create wifi environemnt variables
#
# additionally outputs ssid, password, admin password and wps pin
#
# outputs:
#   ./name.wifi.env
#   ./name.wifi.ssid.pub
#   ./name.wifi.pass
#   ./name.wifi.admin
#   ./name.wifi.wps
def "main secrets wifi env" [name: string]: nothing -> nothing {
  let ssid = $"pidgeon-(random chars --length 16)"
  $ssid | try { save $"($name).wifi.ssid.pub" }
  chmod 644 $"($name).wifi.ssid.pub"

  let pass = random chars --length 32
  $ssid | try { save $"($name).wifi.pass" }
  chmod 600 $"($name).wifi.pass"

  let admin = random chars --length 32
  $ssid | try { save $"($name).wifi.admin" }
  chmod 600 $"($name).wifi.admin"

  let wps = (0..(4 - 1)) | each { |_| random int 0..9 } | str join ""
  $ssid | try { save $"($name).wifi.wps" }
  chmod 600 $"($name).wifi.wps"

  let wifi_env = $"WIFI_SSID=\"(open --raw $"($name).wifi.ssid.pub")\"
WIFI_PASS=\"(open --raw $"($name).wifi.pass")\""
  $wifi_env | try { save $"($name).wifi.env" }
  chmod 600 $"($name).wifi.env"
}

def "main secrets pidgeon env" [name: string] -> {
    
}

# create a linux user password
#
# outputs:
#   ./name.pass.pub
#   ./name.pass
def "main secrets pass" [name: string, length: int = 8]: nothing -> nothing {
  let pass = random chars --length $length
  $pass | try { save $"($name).pass" }
  chmod 600 $"($name).pass"

  let encrypted = $pass | mkpasswd --stdin
  $encrypted | try { save $"($name).pass.pub" }
  chmod 644 $"($name).pass.pub"
}

# create a random numeric pin of specified length
#
# outputs:
#   ./name.pin
def "main secrets pin" [name: string, --length: number = 8]: nothing -> nothing {
  let pin = (0..($length - 1)) | each { |_| random int 0..9 } | str join ""
  $pin | try { save $"($name).pin" }
  chmod 600 $"($name).pin"
}

# create a random alphanumeric key of specified length
#
# outputs:
#   ./name.key
def "main secrets key" [name: string, --length: number = 32]: nothing -> nothing {
  let key = random chars --length $length
  $key | try { save $"($name).key" }
  chmod 600 $"($name).key"
}

# create a random alphanumeric id of specified length
#
# outputs:
#   ./name.id.pub
def "main secrets id" [name: string, --length: int = 16]: nothing -> nothing  {
  let id = random chars --length $length
  $id | try { save $"($name).id.pub" }
  chmod 644 $"($name).id.pub"
}
