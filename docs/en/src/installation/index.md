# Installation

The installation of Pidgeon involves several steps, each of which is detailed on
its own page. Here's an overview of the process:

1. **Generate Secrets**: A script in the repository uses `sops` and `openssl` to
   generate secrets for a specific Raspberry Pi. This step is crucial for
   securing communication between the device and the server.

2. **Create ISO Image**: Another script in the repository uses `nix build` to
   create an ISO image for the device. This image contains the Pidgeon
   application and all its dependencies.

3. **Inject Secret key**: The secret key generated in step 1 is injected into
   the image using a script in the repository. The secret key is used to decrypt
   the secrets generated in step 1 during boot.

4. **Assemble the Device**: The ISO image is flashed onto a 1TB SSD using `dd`.
   The SSD is then plugged into a USB port of the Raspberry Pi, and the power
   USB-C cable is plugged in.
