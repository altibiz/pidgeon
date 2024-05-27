# Secrets generation

With command:

```bash
scripts/mksecrets <cloud_domain> <network_ip_range_start> <network_ip_range_end>
```

, this script generates a set of secrets for a specific Raspberry Pi device
using OpenSSL and SOPS, and prepares them for injection into an ISO image.

## Steps

1. The script takes three arguments: `cloud_domain`, `network_ip_range_start`,
   and `network_ip_range_end`. It checks if these arguments are provided, else
   it exits with an error message.

2. It sets up directories for storing secrets and temporary secrets.

3. It generates a unique ID for the device and checks if secrets for this ID
   already exist. If they do, it exits with an error message.

4. It defines several helper functions for generating different types of secrets
   (IDs, keys, passwords, age keys, SSH keys, SSL certificates). These secrets
   are generated using OpenSSL, age-keygen, ssh-keygen, and mkpasswd.

5. It generates secrets for various components (altibiz, api, pidgeon, secrets,
   postgres) using these helper functions.

6. It creates a PostgreSQL script for setting up the database and users with
   their respective passwords.

7. It creates an environment file (pidgeon.env) with various configuration
   settings, including the database URL, cloud domain, API key, network IP
   range, etc.

8. It creates a YAML file (secrets.yaml) with the generated secrets.

9. It encrypts the secrets.yaml file using SOPS and the public age keys of
   altibiz, pidgeon, and secrets. The encrypted file (secrets.enc.yaml) is then
   copied to the src/flake/enc directory with the device's unique ID as its
   name.

After the script is done, the generated secrets can then be injected into an ISO
image for the Raspberry Pi device.
