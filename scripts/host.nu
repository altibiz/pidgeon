#!/usr/bin/env nu

use std
use ./static.nu *
let root = $env.FILE_PWD | path dirname
let hosts_dir = [ $root "src" "flake" "host" ] | path join
let self = [ $root "scripts" "host.nu" ] | path join

# host configuration, secret, image generation script
#
# start with `host create` if you're starting from scratch
# when adding new secrets for all hosts use `host generate`
# when writing images to disks use `host write`
# writing is a separate command because it requires root privileges
#
# NOTE: any generated secret will not trump
# a previously generated secret
def "main" [ ] {
  let hosts = ls $hosts_dir
    | where $it.type == "dir"
    | get name
    | path basename
    | parse "pidgeon-{id}"
    | get id

  print $"Hello, ($env.USER)!"
  print ""
  print "You are using the pidgeon host CLI."
  print ""

  mut command = "create"
  if (($hosts | length) > 0) {
    print "Please select the command to execute."
    print ""
    print "When creating a new host use the `create` command."
    print "When adding new secrets for hosts use the `generate` command."
    print "When writing images to disks use the `write` command."
    print "When connecting to hosts use the `connect` command."
    print "When deploying to hosts use the `deploy` command."
    print ""
    print "NOTE: the `write` command requires root privileges"
    $command = (gum choose
      --header "Command:"
      create generate write connect deploy)
  } else {
    print "It seems you have not used this CLI yet."
    print "To use all functionality of this CLI you will need to create your first host."
  }
  print ""

  if $command == "create" {
    if (($hosts | length) > 0) {
      print "You selected the `create` command."
      print ""
    }

    mut secrets_dir = ""
    if ("secrets" | path exists) {
      try {
        gum confirm "Is it okay if I use the secrets directory to get secrets from?"
        $secrets_dir = "secrets"
      }
    }
    if ($secrets_dir == "") {
      print "Please select the secrets directory."
      $secrets_dir = (gum choose 
        --header "Secrets directory:"
        ...(ls).name)
    }
    print $"You selected the '($secrets_dir)' directory for secrets."
    print ""

    mut images_dir = ""
    if ("images" | path exists) {
      try {
        gum confirm "Is it okay if I use the images directory to get images from?"
        $images_dir = "images"
      }
    }
    if ($images_dir == "") {
      print "Please select the images directory."
      $images_dir = (gum choose
        --header "Images directory:"
        ...(ls).name)
    }
    print $"You selected the '($images_dir)' directory for images."
    print ""

    mut wifi_host = ""
    if (($hosts | length) > 0) {
      try {
        gum confirm "Would you like to borrow wifi secrets from another host?"
        print "Please select the host to borrow wifi secrets from."
        $wifi_host = (gum choose --header "Host:" ...($hosts))
      }
      if ($wifi_host | is-not-empty) {
        print $"You selected the '($wifi_host)' host for wifi secret sharing."
      } else {
        print $"You did not select a host for wifi secret sharing."
      }
      print ""
    }

    mut id = ""
    try {
      gum confirm "Would you like to set the id of the host instead of generating it?"
      print "Please write in the id of the host."
      $id = (gum input --placeholder "Id...")
    }
    if ($id | is-not-empty) {
      print $"You wrote in '($id)' for the host id."
    } else {
      $id = random chars --length 32
      print $"You did not write an id for the host id."
      print $"The '($id)' id has been generated for you."
    }
    print ""

    print "I am ready to start the `create` command now."
    print ""
    print $"I will initialize a new host configuration in '($hosts_dir)/($id)'."
    if (ls $secrets_dir | is-empty) {
      print $"I will initialize secrets in '($secrets_dir)'."
    }
    print $"I will generate secrets for the new host in '($secrets_dir)/($id)'."
    print $"I will generate an image for the new host in '($images_dir)/($id).img'."
    print ""
    print "This will take some time."
    print ""
    try {
      gum confirm "Are you ready to create the host configuration, secrets and image?"
    } catch {
      print "You were not ready to start the command."
      print "Exiting..."
      print ""
      exit 1
    }

    print "Starting the `create` command now."
    let wifi_arg = if ($wifi_host | is-empty) { "" } else { $" --wifi-from ($wifi_host)" }
    let command = $"($self) create ($secrets_dir) ($images_dir)($wifi_arg) --id ($id)"
    print "\n"
    spin "create" $command
    print "\n"

    print $"Host successfully created with the id: '($id)'."
    print ""
  } else if $command == "generate" {
    print "You selected the `generate` command."
    print ""

    mut secrets_dir = ""
    if ("secrets" | path exists) {
      try {
        gum confirm "Is it okay if I use the secrets directory to get secrets from?"
        $secrets_dir = "secrets"
      }
    }
    if ($secrets_dir == "") {
      print "Please select the secrets directory."
      $secrets_dir = (gum choose 
        --header "Secrets directory:"
        ...(ls).name)
    }
    print $"You selected the '($secrets_dir)' directory for secrets."
    print ""

    mut images_dir = ""
    if ("images" | path exists) {
      try {
        gum confirm "Is it okay if I use the images directory to get images from?"
        $images_dir = "images"
      }
    }
    if ($images_dir == "") {
      print "Please select the images directory."
      $images_dir = (gum choose
        --header "Images directory:"
        ...(ls).name)
    }
    print $"You selected the '($images_dir)' directory for images."
    print ""

    mut wifi_host = ""
    if (($hosts | length) > 1) {
      try {
        gum confirm "Would you like to borrow wifi secrets from another host?"
        print "Please select the host to borrow wifi secrets from."
        $wifi_host = (gum choose --header "Host:" ...($hosts))
      }
      if ($wifi_host | is-not-empty) {
        print $"You selected the '($wifi_host)' host for wifi secret sharing."
      } else {
        print $"You did not select a host for wifi secret sharing."
      }
      print ""
    }

    print "Please pick an existing host id."
    let id = (gum choose --header "Id:" ...($hosts | where $it != $wifi_host))
    print $"You chose the '($id)' host."
    print ""

    mut renew = false;
    try {
      gum confirm "Would you like to renew VPN and database certificates?"
      $renew = true;
    }
    if $renew {
      print "You chose to renew VPN and database certificates."
    } else {
      print "You chose not to renew VPN and database certificates."
    }
    print ""

    print "I am ready to start the `generate` command now."
    print ""
    print $"I will generate new secrets for host in the '($secrets_dir)/($id)'"
    print $"I will generate the image for the host in the '($images_dir)/($id).img'"
    print ""
    print "This will take some time."
    print ""
    try {
      gum confirm "Are you ready to generate the host secrets and image?"
    } catch {
      print ""
      print "You were not ready to start the command."
      print "Exiting..."
      print ""
      exit 1
    }
    print ""

    print "Starting the `generate` command now."
    let wifi_arg = if ($wifi_host | is-empty) { "" } else { $" --wifi-from ($wifi_host)" }
    let renew_arg = if $renew { " --renew" } else { "" }
    let command = $"nu ($self) generate ($id) ($secrets_dir) ($images_dir)($wifi_arg)($renew_arg)"
    print "\n"
    spin "generate" $command
    print "\n"

    print $"Host successfully generated with the id: '($id)'."
    print ""
  } else if $command == "write" {
    print "You selected the `write` command."
    print "This command requires root privileges."
    print ""

    mut image = ""
    mut images_dir = ""
    if ("images" | path exists) {
      try {
        gum confirm "Is it okay if I use the images directory to get images from?"
        $images_dir = "images"
      }
    }
    if ($images_dir == "") {
      print "Please select the images directory."
      $images_dir = (gum choose
        --header "Images directory:"
        ...(ls).name)
    }
    print "Please select the origin image."
    $image = (gum choose --header "Image:" ...(ls $images_dir).name)
    print $"You selected the '($image)' image for the original image."
    print ""

    print "Please select the destination disk."
    let destination = (gum choose
      --header "Disk:"
      ...(glob /dev/sd*[!0-9])
      ...(glob /dev/nvme*n[!p]))
    print $"You selected the '($destination)' disk for the destination."
    print ""

    print "I am ready to start the `write` command now."
    print ""
    print $"I will write '($image)' to '($destination)'."
    print ""
    print "This might take some time."
    print "Don't go away right away because there will likely be a sudo password prompt."
    print ""
    try {
      gum confirm "Are you ready to write the host image?"
    } catch {
      print ""
      print "You were not ready to start the command."
      print "Exiting..."
      print ""
      exit 1
    }

    print "Starting the `write` command now."
    let command = $"nu ($self) write ($image) ($destination)"
    print "\n"
    nu -c $command
    print "\n"

    print $"Image ($image) successfully written to ($destination)."
  } else if $command == "connect" {
    print "You selected the `connect` command."
    print ""

    mut secrets_dir = ""
    if ("secrets" | path exists) {
      try {
        gum confirm "Is it okay if I use the secrets directory to get secrets from?"
        $secrets_dir = "secrets"
      }
    }
    if ($secrets_dir == "") {
      print "Please select the secrets directory."
      $secrets_dir = (gum choose 
        --header "Secrets directory:"
        ...(ls).name)
    }
    print $"You selected the '($secrets_dir)' directory for secrets."
    print ""

    print "Please pick an existing host id."
    let id = (gum choose --header "Id:" ...($hosts))
    print $"You chose the '($id)' host."
    print ""

    print "I am ready to start the `connect` command now."
    print ""
    print $"I will connect to host '($id)'."
    try {
      gum confirm "Are you ready to connect to the host?"
    } catch {
      print ""
      print "You were not ready to start the command."
      print "Exiting..."
      print ""
      exit 1
    }

    print "Starting the `connect` command now."
    let command = $"nu ($self) connect ($id) ($secrets_dir)/($id)/($id).ssh.key"
    print "\n"
    nu -c $command
    print "\n"

    print $"Disconnected from host '($id)'"
  } else if $command == "deploy" {
    print "You selected the `deploy` command."
    print ""

    mut secrets_dir = ""
    if ("secrets" | path exists) {
      try {
        gum confirm "Is it okay if I use the secrets directory to get secrets from?"
        $secrets_dir = "secrets"
      }
    }
    if ($secrets_dir == "") {
      print "Please select the secrets directory."
      $secrets_dir = (gum choose 
        --header "Secrets directory:"
        ...(ls).name)
    }
    print $"You selected the '($secrets_dir)' directory for secrets."
    print ""

    print "Please pick an existing host id."
    let id = (gum choose --header "Id:" ...($hosts))
    print $"You chose the '($id)' host."
    print ""

    print "I am ready to start the `deploy` command now."
    print ""
    print $"I will deploy to host '($id)'."
    try {
      gum confirm "Are you ready to deploy to the host?"
    } catch {
      print ""
      print "You were not ready to start the command."
      print "Exiting..."
      print ""
      exit 1
    }

    print "Starting the `deploy` command now."
    let command = $"nu ($self) deploy ($id) ($secrets_dir)/($id)/($id).ssh.key"
    print "\n"
    nu -c $command
    print "\n"

    print $"Deployed to host '($id)'"
  }
}

def spin [name: string, command: string]: nothing -> nothing {
  let temp = mktemp -t
  $command | save -f $temp
  chmod 700 $temp
  (gum spin nu
    $temp
    --title $"Please wait for `($name)` to finish..."
    --show-error)
  rm -f $temp
}

# create host configuration, secrets and image
#
# optionally borrow wifi secrets from another host
def "main create" [
  # directory in which to generate secrets
  secrets_dir: string,
  # directory in which to generate image
  images_dir: string,
  # id of host to borrow wifi secrets from
  --wifi-from: string,
  # set the id of the host instead of randomly generating it
  --id: string
]: nothing -> string {
  let pwd = pwd

  let id = main init --id $id

  cd $secrets_dir
  let secrets = (main
    secrets
    generate
    $id
    --wifi-from
    $wifi_from)
  cd $pwd

  cd $images_dir
  let image = main image generate $id
  cd $pwd

  main image inject $secrets $image

  $id
}

# regenerate host secrets and image
#
# optionally borrow wifi secrets from another host
def "main generate" [
  # id of host to generate secrets and image for
  id: string,
  # directory in which to generate secrets
  secrets_dir: string,
  # directory in which to generate image
  images_dir: string,
  # id of host to borrow wifi secrets from
  --wifi-from: string,
  # renew certificates
  --renew
]: nothing -> nothing {
  let pwd = pwd

  cd $secrets_dir
  let secrets = (main secrets generate
    $id
    --wifi-from $wifi_from
    --renew)
  cd $pwd

  cd $images_dir
  let image = main image generate $id
  cd $pwd

  main image inject $secrets $image
}

# write image to specified destination
#
# basically a sane wrapper over the `dd` and `sync` commands
def "main write" [
  image: string,
  destination: string
]: nothing -> nothing {
  (sudo dd
    $"if=($image)"
    $"of=($destination)"
    bs=4M
    conv=sync,noerror
    status=progress
    oflag=direct)
  sync
}

# connect to specified host
#
# basically a wrapper over the `ssh` command
#
# assumes a vpn connection is running
def --wrapped "main connect" [
  id: string,
  key: string,
  ...args
]: nothing -> nothing {
  let $hosts = static hosts $hosts_dir
  let $host = $hosts | get $"pidgeon-($id)"

  (ssh
    $"altibiz@($host.vpn.ip)"
    -i $key
    ...($args))
}

# deploy specified host
#
# basically a wrapper over `deploy` command
#
# assumes a vpn connection is running
def --wrapped "main deploy" [
  id: string,
  key: string,
  ...args
]: nothing -> nothing {
  (deploy
    --interactive
    ...($args)
    --ssh-opts $"-i ($key)"
    --
    $"($root)#pidgeon-($id)-aarch64-linux")
}


# initialize host with empty configuration
# and an available ip address
def "main init" [--id: string]: nothing -> string {
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
        | values
        | get vpn.ip
        | each { |x| 
            let p = $x
              | parse "{a}.{b}.{c}.{d}"
              | each { |x|
                  {
                    d: ($x.d | into int),
                    c: ($x.c | into int),
                    b: ($x.b | into int),
                    a: ($x.a | into int)
                   }
                }
              | first
            {
              parsed: $p
              sum: ($p.d * 2 ** 0
                + $p.c * 2 ** 1
                + $p.b * 2 ** 2
                + $p.a * 2 ** 3)
              ip: $x
            }
          }
        | sort-by sum
        | last
    }

  let next_ip = if ($last_ip == null) {
    "10.8.0.10"
  } else if ($last_ip.parsed.d == 254) {
    $"($last_ip.parsed.a).($last_ip.parsed.b).($last_ip.parsed.c + 1).(0)"
  } else {
    $"($last_ip.parsed.a).($last_ip.parsed.b).($last_ip.parsed.c).($last_ip.parsed.d + 1)"
  }

  echo '{ }' | try { save $"($host_dir)/config.nix" }
  { vpn: { ip: $next_ip } } | to json | try { save $"($host_dir)/static.json" }

  let pwd = pwd
  cd $root
  git add $"($host_dir)/config.nix"
  git add $"($host_dir)/static.json"
  cd $pwd

  $id
}

# generate secrets for a specified host
def "main secrets generate" [
  id: string,
  --wifi-from: string,
  --renew
]: nothing -> string {
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
  if $renew {
    main secrets vpn key $id ../shared --renew
    main secrets db key $id ../shared --renew
  } else {
    main secrets vpn key $id ../shared
    main secrets db key $id ../shared
  }
  main secrets db sql $id
  if ($wifi_from | is-empty) {
    main secrets wifi env $id
  } else {
    glob $"../($wifi_from)/($wifi_from).wifi.*"
      | each { |x|
          let suffix = $x
            | path basename
            | parse "{id}.wifi.{suffix}"
            | get suffix
            | first
          try { cp -n $x $"($id).wifi.($suffix)" }
        }
  }
  main secrets pidgeon env $id
  main secrets scrt key $id

  mkdir val
  cd val

  cp -f $"../($id).ssh.key.pub" altibiz.ssh.pub
  cp -f $"../($id).pass.pub" altibiz.pass.pub
  cp -f $"../($id).pidgeon.env" pidgeon.env
  cp -f $"../($id).db.key" postgres.crt
  cp -f $"../($id).db.key.pub" postgres.crt.pub
  cp -f $"../($id).db.sql" postgres.sql
  cp -f $"../($id).wifi.env" wifi.env
  cp -f ../../shared.vpn.ca.pub nebula.ca.pub
  cp -f $"../($id).vpn.key" nebula.crt
  cp -f $"../($id).vpn.key.pub" nebula.crt.pub
  cp -f $"../($id).scrt.key.pub" .

  main secrets scrt val $id

  cp -f $"($id).scrt.val.pub" ../
  cp -f $"($id).scrt.val" ../

  cd ../
  cd ../

  try { cp -n $"($secrets_dir)/($id).scrt.key.pub" . }
  try { cp -n $"($secrets_dir)/($id).scrt.key" . }
  cp -f $"($secrets_dir)/val/($id).scrt.val.pub" .
  cp -f $"($secrets_dir)/val/($id).scrt.val" .
  cp -f $"($id).scrt.val.pub" $"($host_dir)/secrets.yaml"

  let pwd = pwd
  cd $root
  git add $"($host_dir)/secrets.yaml"
  cd $pwd

  $"(pwd)/($id).scrt.key"
}

# generate a specified hosts' image
# outputs the path to the generated image
def "main image generate" [id: string]: nothing -> string {
  let compressed = ls (nixos-generate
    --system "aarch64-linux"
    --format "sd-aarch64"
    --flake $"($root)#pidgeon-($id)-aarch64-linux"
    | path dirname --num-levels 2
    | path join "sd-image")
    | get name
    | first
  unzstd $compressed -o $"./($id)-temp.img"
  mv -f  $"./($id)-temp.img" $"./($id).img" 
  chmod 644 $"./($id).img"
  rm -f result
  $"(pwd)/($id).img"
}

# inject secrets key into a host image
#
# uses libguestfs
def "main image inject" [
  secrets_key: string,
  image: string
]: nothing -> nothing {
  let commands = $"run
mount /dev/sda2 /
mkdir /root
chmod 700 /root
upload ($secrets_key) /root/secrets.age
chmod 400 /root/secrets.age
exit"

  echo $commands | guestfish --rw -a $image
}

# create a secret key
#
# assumes sops is used
#
# outputs:
#   ./name.scrt.key.pub
#   ./name.scrt.key
def "main secrets scrt key" [name: string]: nothing -> nothing {
  age-keygen err> (std null-device) out> $"($name)-temp.scrt.key"
  try { mv -n $"($name)-temp.scrt.key" $"($name).scrt.key" }
  rm -f $"($name)-temp.scrt.key"
  chmod 600 $"($name).scrt.key"

  open --raw $"($name).scrt.key"
    | (age-keygen -y
      err> (std null-device)
      out> $"($name)-temp.scrt.key.pub")
  mv -f $"($name)-temp.scrt.key.pub" $"($name).scrt.key.pub"
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
  let vals = ls $env.PWD
    | where { |x| $x.type == "file" }
    | where { |x| 
        let basename = $x.name | path basename
        return (
          not ($basename | str ends-with ".scrt.val")
          and not ($basename | str ends-with ".scrt.val.pub")
          and not ($basename | str ends-with ".scrt.key")
          and not ($basename | str ends-with ".scrt.key.pub")
        )
      }
    | each { |x|
        let content = open --raw $x.name
          | str trim
          | str replace --all "\n" "\n  "
        return $"($x.name | path basename): |\n  ($content)" 
      }
    | str join "\n"
  $vals | save -f $"($name).scrt.val"
  chmod 600 $"($name).scrt.val"

  let keys = ls $env.PWD
    | where { |x| $x.type == "file" }
    | where { |x| 
        let basename = $x.name | path basename
        return (
          ($basename | str ends-with ".scrt.key.pub")
        )
      }
    | each { |x| open --raw $x.name }
    | str join ","

  (sops encrypt $"($name).scrt.val"
    --input-type yaml
    --age $keys
    --output $"($name)-temp.scrt.val.pub"
    --output-type yaml)

  mv -f $"($name)-temp.scrt.val.pub" $"($name).scrt.val.pub"
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
  ssh-keygen -q -a 100 -t ed25519 -N "" -C $name -f $"($name)-temp.ssh.key"

  try { mv -n $"($name)-temp.ssh.key.pub" $"($name).ssh.key.pub" }
  rm -f $"($name)-temp.ssh.key.pub"
  chmod 644 $"($name).ssh.key.pub"

  try { mv -n $"($name)-temp.ssh.key" $"($name).ssh.key" }
  rm -f $"($name)-temp.ssh.key"
  chmod 600 $"($name).ssh.key"
}

# create the nebula vpn ca
#
# outputs:
#   ./name.vpn.ca.pub
#   ./name.vpn.ca
def "main secrets vpn ca" [name: string]: nothing -> nothing {
  (nebula-cert ca
    -name $name
    -duration $"(365 * 24 * 100)h"
    -out-crt $"($name)-temp.vpn.ca.pub"
    -out-key $"($name)-temp.vpn.ca")

  try { mv -n $"($name)-temp.vpn.ca.pub" $"($name).vpn.ca.pub" }
  rm -f $"($name)-temp.vpn.ca.pub"
  chmod 644 $"($name).vpn.ca.pub"

  try { mv -n $"($name)-temp.vpn.ca" $"($name).vpn.ca" }
  rm -f $"($name)-temp.vpn.ca"
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
def "main secrets vpn key" [
  name: string,
  ca: path,
  --renew
]: nothing -> nothing {
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
    -ip $ip
    -out-crt $"($name)-temp.vpn.key.pub"
    -out-key $"($name)-temp.vpn.key")

  if $renew {
    mv -f $"($name)-temp.vpn.key.pub" $"($name).vpn.key.pub"
  } else {
    try { mv -n $"($name)-temp.vpn.key.pub" $"($name).vpn.key.pub" }
    rm -f $"($name)-temp.vpn.key.pub"
  }
  chmod 644 $"($name).vpn.key.pub"

  if $renew {
    mv -f $"($name)-temp.vpn.key" $"($name).vpn.key"
  } else {
    try { mv -n $"($name)-temp.vpn.key" $"($name).vpn.key" }
    rm -f $"($name)-temp.vpn.key"
  }
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
    -out $"($name)-temp.db.ca")

  (openssl req -x509
    -key $"($name)-temp.db.ca"
    -out $"($name)-temp.db.ca.pub"
    -subj $"/CN=($name)"
    -days 3650)

  try { mv -n $"($name)-temp.db.ca.pub" $"($name).db.ca.pub" }
  rm -f $"($name)-temp.db.ca.pub"
  chmod 644 $"($name).db.ca.pub"

  try { mv -n $"($name)-temp.db.ca.srl" $"($name).db.ca.srl" }
  rm -f $"($name)-temp.db.ca.srl"
  chmod 644 $"($name).db.ca.srl"

  try { mv -n $"($name)-temp.db.ca" $"($name).db.ca" }
  rm -f $"($name)-temp.db.ca"
  chmod 600 $"($name).db.ca"
}

# create database ssl keys
# signed by a previously generated db ca
#
# assumes that the database uses openssl keys
#
# outputs:
#   ./name.db.key.pub
#   ./name.db.key
def "main secrets db key" [
  name: string,
  ca: path,
  --renew
]: nothing -> nothing {
  (openssl genpkey -algorithm ED25519
    -out $"($name)-temp.db.key")

  (openssl req -new
    -key $"($name)-temp.db.key"
    -out $"($name)-temp.db.key.req"
    -subj $"/CN=($name)")
  (openssl x509 -req
    -in $"($name)-temp.db.key.req"
    -CA $"($ca).db.ca.pub"
    -CAkey $"($ca).db.ca"
    -CAcreateserial
    -out $"($name)-temp.db.key.pub"
    -days 3650) err>| ignore

  rm -f $"($name)-temp.db.key.req"

  if $renew {
    mv -f $"($name)-temp.db.key" $"($name).db.key" 
  } else {
    try { mv -n $"($name)-temp.db.key" $"($name).db.key" }
    rm -f $"($name)-temp.db.key"
  }
  chmod 600 $"($name).db.key"

  if $renew {
    mv -f $"($name)-temp.db.key.pub" $"($name).db.key.pub"
  } else {
    try { mv -n $"($name)-temp.db.key.pub" $"($name).db.key.pub" }
    rm -f $"($name)-temp.db.key.pub"
  }
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
    $key | try { save  $"($name).db.user" }
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
  $pass | try { save $"($name).wifi.pass" }
  chmod 600 $"($name).wifi.pass"

  let admin = random chars --length 8
  $admin | try { save $"($name).wifi.admin" }
  chmod 600 $"($name).wifi.admin"

  let wps = (0..(4 - 1)) | each { |_| random int 0..9 } | str join ""
  $wps | try { save $"($name).wifi.wps" }
  chmod 600 $"($name).wifi.wps"

  let wifi_env = $"WIFI_SSID=\"(open --raw $"($name).wifi.ssid.pub")\"
WIFI_PASS=\"(open --raw $"($name).wifi.pass")\""
  $wifi_env | save -f $"($name).wifi.env"
  chmod 600 $"($name).wifi.env"
}

# generate pidgeon environment
#
# expects network ip range start in `PIDGEON_NETWORK_IP_RANGE_START`
# with "192.168.1.0 as default value
#
# expects network ip range end in `PIDGEON_NETWORK_IP_RANGE_END`
# with "192.168.1.0 as default value
#
# expects cloud domain provided in `PIDGEON_NAME_CLOUD_DOMAIN`
def "main secrets pidgeon env" [name: string] -> {
  let network_ip_range_start_key = $"PIDGEON_NETWORK_IP_RANGE_START"
  let network_ip_range_start = $env
    | default "192.168.1.255" $network_ip_range_start_key
    | get $network_ip_range_start_key
  let network_ip_range_end_key = $"PIDGEON_NETWORK_IP_RANGE_END"
  let network_ip_range_end = $env
    | default "192.168.1.0" $network_ip_range_end_key
    | get $network_ip_range_end_key
  let cloud_domain_key = $"PIDGEON_CLOUD_DOMAIN"
  let cloud_domain = $env | default null $cloud_domain_key | get $cloud_domain_key
  if ($cloud_domain | is-empty) {
    error make {
      msg: "expected api key provided via PIDGEON_CLOUD_DOMAIN"
    }
  }

  let port = [ $root "docker-compose.yml" ]
    | path join
    | open
    | get services.postgres.ports
    | filter { |x| ($x | split row ":" | get 1) == "5432" }
    | split row ":"
    | get 0


  let pidgeon_env = $"PIDGEON_DB_DOMAIN=\"localhost\"
PIDGEON_DB_PORT=\"($port)\"
PIDGEON_DB_USER=\"pidgeon\"
PIDGEON_DB_PASSWORD=\"(open --raw $"($name).pidgeon.db.user")\"
PIDGEON_DB_NAME=\"pidgeon\"

PIDGEON_CLOUD_DOMAIN=\"($cloud_domain)\"
PIDGEON_CLOUD_API_KEY=\"(open --raw $"($name).key")\"
PIDGEON_CLOUD_ID=\"pidgeon-($name)\"

PIDGEON_NETWORK_IP_RANGE_START=\"($network_ip_range_start)\"
PIDGEON_NETWORK_IP_RANGE_END=\"($network_ip_range_end)\""
  $pidgeon_env | save -f $"($name).pidgeon.env"
  chmod 600 $"($name).pidgeon.env"
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
