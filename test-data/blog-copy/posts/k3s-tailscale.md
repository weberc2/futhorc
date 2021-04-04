---
Title: K3s + Tailscale
Date: 2020-12-30
Tags: [Homelab]
---

I've recently been working on my Raspberry Pi Kubernetes cluster. I also use
[Tailscale][0] for my home VPN (because it's performant and absurdly easy to
setup and configure). I wanted to run Kubernetes services on my VPN using
private DNS names (e.g., `foo.local`) and addresses from the Tailscale address
space (e.g., `100.*`) as opposed to the host network address space (e.g.,
`192.168.*`).

<!-- more -->

Since I'm using [Klipper LB][1], the default load balancer controller for
[k3s][2], services' external IP addresses are always the IP addresses of a node
in the cluster, and since each node is running on the Tailscale network,
configuring DNS is pretty straightforward: get the external IP address of the
service (in the `192.168.*` address space), figure out what host that IP
address corresponds to, and then figure out the Tailscale IP address (from the
`100.*` address space) and create the A record mapping a DNS name to that
Tailscale IP address.

However, because these external service IP addresses change from time to time
and to satiate my compulsion to automate everything, I'm running a DNS
controller--a Kubernetes service that watches the Kubernetes cluster for
changes to Ingresses and Services and CRUDs DNS records accordingly. So when a
service (or ingress) is added to the cluster with an external IP address, the
controller will fetch the DNS name for the service from an attribute and create
an A record mapping that DNS name to the service's external IP address.

Since that IP address is in the host network IP space (`192.168.*`), the DNS
won't resolve on hosts that are inside of the VPN but outside of the physical
network (e.g., my cell phone is connected to the VPN over its 4G network). So
either the controller needs to be Tailscale-aware (determine the host for the
`192.168.*` IP address and then determine the Tailscale IP address for that
host) or Klipper needs to assign services external IP addresses from the
`100.*` address space instead of the `192.168.*` address space.

# Solution: Configure K3s to use the Tailscale network

The latter seemed easier and generally keeps my DNS controller decoupled from
the details of my VPN and Load Balancer controller, so I went that route. I use
[`k3sup`][3] to install k3s on my nodes, so my invocation `install` and `join`
(for installing k3s on the master and worker nodes, respectively) looks like
this:

```bash
# Install k3s; merge the kubeconfig with the user's home kubeconfig instead of
# sporadically dropping kubeconfig files all over.
$ k3sup install \
    --ip "$HOST" \
    --user "$USER" \
    --ssh-key "$PRIVKEY" \
    --k3s-extra-args "--flannel-iface tailscale0 \
        --advertise-address $TAILSCALE_ADDR \
        --node-ip $TAILSCALE_ADDR \
        --node-external-ip $TAILSCALE_ADDR" \
    --k3s-channel latest \
    --context rpis \
    --local-path $HOME/.kube/config \
    --merge
```

And

```bash
$ k3sup join \
    --ip "$HOST" \
    --server-ip "$MASTER_NODE" \
    --user "$USER" \
    --ssh-key "$PRIVKEY" \
    --k3s-extra-args "--flannel-iface tailscale0 \
        --node-ip $TAILSCALE_ADDR \
        --node-external-ip $TAILSCALE_ADDR" \
    --k3s-channel latest
```

For the purposes of this article, the noteworthy bits are the
`--k3s-extra-args`. This string is plumbed through to the `k3s server` and
`k3s agent` invocations in the resulting systemd units, and they tell k3s to
use the Tailscale network interface and IP addresses. Effectively, as far as
k3s is concerned, these nodes are all connected via Tailscale rather than the
host network, so in theory (i.e., I haven't confirmed this yet) I should be
able to add a node in the same way even if it's not on the same host network
(e.g., an EC2 instance running in an AWS VPC).

# Alternative Solution: Bridge Host and Tailscale Networks

Another solution would have been to bridge the host network with the Tailscale
network such that a device running on a different physical network (but still
on the VPN) could resolve addresses in the `192.168.*` address space (and vice
versa)--Tailscale has documentation for that [here][4].

I like that a little less because it requires a different Tailscale
configuration for one node (that node must be configured to advertise `100.*`
addresses on the physical network and `192.168.*` addresses on the Tailscale
network, as well as configuring subnets and disabling key rotation. Further,
traffic that needs to cross this boundary must be routed through a particular
node, adding a runtime cost (no real idea if the runtime cost exceeds Tailscale
overhead). Mostly, it just adds a fair amount of complexity at the networking
level, where I'm least comfortable debugging should something go wrong.


[0]: https://tailscale.com/
[1]: https://github.com/k3s-io/klipper-lb
[2]: https://k3s.io
[3]: https://github.com/alexellis/k3sup
[4]: https://tailscale.com/kb/1019/subnets
