---
Title: Working around ext4 on MacOS
Date: 2021-01-23
Tags: [macOS, ext4]
---

One of the maddening things about MacOS is that it lacks ext4 file system
support. This is a bummer because I use ext4 on many of my external hard
drives as well as boot volumes for various headless Linux machines. Once in a
while, these boot drives become corrupted (due to user error) and I find myself
wanting to mount the file system on another system to repair the error. Ideally
I can just pop it in a SATA<->USB adapter and mount it on my Mac, but alas...

<!-- more -->

There is a read-only ext4 FUSE driver for MacOS installable via Homebrew, but
that's not going to help us repair a bad volume. I could install some virtual
machine manager, bootstrap a machine, figure out how to share the host USB
devices, etc, but that seems like a lot of work. If I recall correctly, there
are also paid ext4 FUSE drivers, but I'm cheap.

What *has* worked for me is connecting the disk to a Raspberry Pi via my
SATA<->USB adapter, SSHing onto the Pi from my MacBook, and mounting the drive
that way. And in case of long-running processes (like reformatting the drive),
to avoid the MacBook falling asleep or anything else that might drop the SSH
session, we can use `nohup` to keep the process alive (e.g., `sudo nohup mkfs
-t ext4 /dev/sda1`).

Of course, SSHing through a Raspberry Pi is only convenient if you have a
Raspberry Pi up and running and configured for SSH already. For everyone else,
Virtual Box is probably the way to go.
