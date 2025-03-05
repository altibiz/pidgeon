# Assembly

This chapter describes the final steps of the installation process, which
involve flashing the ISO image onto a 1TB SSD, and assembling the Raspberry Pi
device.

## Flashing the ISO Image

To flash the ISO image onto the SSD, you can use the `dd` command on Linux or a
program like Rufus on Windows.

### Linux

On Linux, you can use the dd command to flash the ISO image onto the SSD. First,
identify the device path of the SSD by running lsblk. Once you have the device
path, you can flash the ISO image with the following command:

```bash
sudo dd if=<iso> of=<device> bs=4M status=progress && sync
```

Replace `<iso>` with the path to the ISO image and `<device>` with the device
path of the SSD. This command writes the ISO image to the SSD block by block and
shows progress information. The sync command is used to ensure all data is
flushed to the device.

### Windows

On Windows, you can use a program like Rufus to flash the ISO image onto the
SSD. Here are the steps:

1. Download and install Rufus from the official website.
2. Plug the SSD into a USB port of your computer.
3. Open Rufus and select the SSD in the 'Device' dropdown.
4. In the 'Boot selection' dropdown, select 'Disk or ISO image' and click the
   'Select' button to choose your ISO file.
5. Click 'Start' to begin the process. Rufus will format the SSD and flash the
   ISO image onto it. Please note that all existing data on the SSD will be
   erased.

## Assembling the Device

After flashing the ISO image onto the SSD, you can assemble the Raspberry Pi
device.

Unplug the SSD from your computer. Plug the SSD into a USB port of the Raspberry
Pi. Plug in the power USB-C cable to power up the Raspberry Pi. The device
should now boot up from the SSD and start running `pidgeon`.
