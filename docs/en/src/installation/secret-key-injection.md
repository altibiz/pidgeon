# Secret Key Injection

With command:

```bash
./inject <iso> <key>
```

, this script injects the secret key into the ISO image for a specific Raspberry
Pi device using the `inject` script in the repository.

## Prerequisites

Before you start, make sure you have generated the ISO image for the device
using the `image` script. The `inject` script requires an ISO image and a secret
key file.

## Usage

The `inject` script takes two arguments: `iso`, which is the path to the ISO
image, and `key`, which is the path to the secret key file. The script checks if
these files exist. If they do not, it exits with an error message.

This is important because we want the scripts to be used by programs on the
device using `nix`, which requires the secrets to be encrypted in the repository
and decrypted on the device on boot.
