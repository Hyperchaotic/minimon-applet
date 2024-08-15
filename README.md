# Minimon COSMIC Applet

A little applet for displaying total CPU load and/or memory usage. Can sit in the panel or Dock. Can display as icons or text.  

Based on the COSMIC Applet template.

![dock-circles](https://github.com/user-attachments/assets/96128ad5-32ac-459b-9f5f-f66357a2c0e0)
![panel-circles](https://github.com/user-attachments/assets/5ad4fa80-d461-4cd3-aa92-ea25a09339d3)
![screenshot-2024-08-15-00-50-20](https://github.com/user-attachments/assets/4a99da4b-326d-4462-8430-154335390096)

![screenshot-2024-08-15-01-02-19](https://github.com/user-attachments/assets/c1e8bc40-d678-44d0-ae6e-e3036102f4a1)

## Install

To install your COSMIC applet, you will need [just](https://github.com/casey/just), if you're on Pop!\_OS, you can install it with the following command:

```sh
sudo apt install just libxkbcommon-dev
```

After you install it, you can run the following commands to build and install your applet:

```sh
just build-release
sudo just install
```

Alternatively generate a deb file for installation:

```sh
just deb
```
and install with:

```sh
sudo dpkg -i <name_of.deb>
```

