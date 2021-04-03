---
Title: "Kubernetes + Raspberry Pi Homelab: Introduction"
Date: 2021-01-03
---

As I alluded to in my [last post][0], I've finally decided to pull the trigger
and build my own homelab: a personal computing environment for playing around
with new tools and approaches for developing or operating software, including
software that is personally useful.

For my homelab, I want to build a Raspberry Pi Kubernetes cluster for hosting
my own applications and experimenting with different tools and approaches for
operating software. However, bare metal (i.e., not running on a public cloud
provider, such as AWS) is a second-class citizen for Kubernetes, so one must
bring their own providers for storage, networking, load balancing, ingress
(roughly "HTTP/layer-7 routing"), and much more.

One day, I have no doubt that there will be Kubernetes distributions targeting
bare metal which are mature, robust, and open source; in the meanwhile, this
series will document my efforts to work around those limitations so that others
can build their own personal cloud platform more easily (or at least know what
they're considering getting into!).

Next time, I'll delve into the hardware I'm using for my cluster.

* [Part I: Hardware][1]

[0]: ./k3s-tailscale.md
[1]: ./homelab-part-i-hardware.md
