---
Title: Blog infrastructure updates
Date: 2020-03-28
---

Another year, another blog update. BitBucket is deprecating their Mercurial
support, and while I really do appreciate Mercurial, it's just easier for me to
keep everything in GitHub than trying to find another Mercurial provider. Also,
GitHub seems to be improving at a pretty rapid pace. So voila, this blog is now
hosted on GitHub. This includes my pet static site generator, [`neon`][0] which
is used to generate this site.

Further, my pet build tool [`builder`][1] is sufficiently far along that I
can use it to automate the building of this static site. `builder` is pretty
cool so far. At some point I will write a dedicated post about it.

[0]: https://github.com/weberc2/neon
[1]: https://github.com/weberc2/builder