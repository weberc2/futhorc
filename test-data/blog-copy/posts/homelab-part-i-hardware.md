---
Title: "Homelab Part I: Hardware"
Date: 2021-01-14
---

For hardware, I settled on Raspberry Pi 4Bs. They support up to 8GB of RAM
(enough power to run the k3s master nodes) and USB 3.0 for fast external SSD
I/O. The only downside of the 4Bs is that they require more power than the 3Bs,
and the same multiport USB power supplies that could support a 3B cluster
couldn't support a 4B cluster. To solve for this, I decided on PoE hats and a
PoE switch. This halves the number of cables that need to be run to each Pi,
which makes the Pi cluster that much more enjoyable and easy on the eyes.

<!-- more -->

Here's my current bill of materials:

Item                   | Price  | Quantity
-----------------------|--------|----------
[Raspberry Pi 4B][0]   | $90    | 2
[4 PoE port switch][1] | $70    | 1
[6 pack cat6 cable][2] | $14    | 1
[PoE Hat][3]           | $25    | 2
[Samsung 500GB SSD][4] | $85    | 1
[M.2 NVMe SSD case][5] | $24    | 1
[Cluster Case][10]     | $19    | 1

I have 2 Raspberry Pi 4Bs and 2 PoE hats, but I expect to procure more of these
incrementally. The 8GB nodes probably aren't necessary, but I wouldn't get 1GB
nodes despite the tempting price point.

# Power

Since I'm using PoE for a power supply, if I exceed the 4 PoE ports allowed by
my PoE switch, I'll have to buy another PoE switch or upgrade. TP-Link, the
manufacturer of my current switch, also makes an [8-port PoE switch][9] (these
switches are a little confusing because they're actually 8-port and 16-port
switches respectively, but only half of the ports are PoE--I'm listing them
here by their PoE port count which is more interesting for our purposes).

# Storage

My storage solution was informed by Jeff Geerling's excellent blog posts:

* [The fastest USB storage options for Raspberry Pi][6]
* [UASP makes Raspberry Pi disk I/O 50% faster][7]

Notably, by using an NVMe-to-USB-3 case that speaks UASP, we can take advantage
of much of an NVMe SSD's performance (note that a SATA SSD is also probably
fast enough to saturate a USB 3.0 connection, but UASP appears to be
significant).

I only have one disk for now, and I'll either simply sync (rclone) that with S3
for backups or I'll get another disk or two and do some sort of replication
(such as RAID or [Longhorn.io][8] or similar) or both.

# Conclusion

This pretty much covers it for the hardware. Next time we'll dig into some
simple automation to allow us to maintainably set up hosts to join the cluster.


[0]: https://www.amazon.com/gp/product/B08DJ9MLHV/ref=ppx_yo_dt_b_asin_title_o06_s00?ie=UTF8&psc=1
[1]: https://www.amazon.com/dp/B01BW0AD1W/?coliid=I2RH40DWH8TU7S&colid=3T0XFW8OAF4UV&psc=1&ref_=lv_ov_lig_dp_it
[2]: https://www.amazon.com/dp/B01IQWGI0O/?coliid=I114GRYN939O82&colid=3T0XFW8OAF4UV&psc=1&ref_=lv_ov_lig_dp_it
[3]: https://www.amazon.com/dp/B07XB5PR9J/?coliid=I1G27N91A9TCKN&colid=9GAEGP20CUK&psc=1&ref_=lv_ov_lig_dp_it
[4]: https://www.amazon.com/dp/B07M7Q21N7/?coliid=I3AXM6RUV7D99C&colid=9GAEGP20CUK&psc=1&ref_=lv_ov_lig_dp_it
[5]: https://www.amazon.com/dp/B07TJT6W8K/?coliid=I4GDI0L3EGR9B&colid=9GAEGP20CUK&psc=1&ref_=lv_ov_lig_dp_it
[6]: https://www.jeffgeerling.com/blog/2020/fastest-usb-storage-options-raspberry-pi
[7]: https://www.jeffgeerling.com/blog/2020/uasp-makes-raspberry-pi-4-disk-io-50-faster
[8]: https://longhorn.io/
[9]: https://www.amazon.com/dp/B0721V1TGV/?coliid=I3MS3H10K66CGU&colid=9GAEGP20CUK&psc=1&ref_=lv_ov_lig_dp_it
[10]: https://www.amazon.com/dp/B07K72STFB/?coliid=IIMX6TFCHY31M&colid=3T0XFW8OAF4UV&psc=1&ref_=lv_ov_lig_dp_it
