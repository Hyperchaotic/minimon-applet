# Minimon COSMIC Applet

A little applet for displaying total CPU load and/or memory usage. Can sit in the panel or Dock. Can display as icons or text.  

Based on the COSMIC Applet template.

![dock-circles](https://github.com/user-attachments/assets/96128ad5-32ac-459b-9f5f-f66357a2c0e0)
![panel-circles](https://github.com/user-attachments/assets/5ad4fa80-d461-4cd3-aa92-ea25a09339d3)
![panel-text](https://github.com/user-attachments/assets/b2fabd77-039a-4250-8130-fec53f1e307e)

## Install

To install your COSMIC applet, you will need [just](https://github.com/casey/just), if you're on Pop!\_OS, you can install it with the following command:

```sh
sudo apt install just
```

After you install it, you can run the following commands to build and install your applet:

```sh
just build-release
sudo just install
```
