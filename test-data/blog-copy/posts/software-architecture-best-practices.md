---
Title: Software architecture best practices
Date: 2015-09-30
---

This is a collection of articles I've found about software development
best-practices. I intend to add to it over time, so don't be surprised if it
changes.

I've known about this blog post for a couple years, and I find myself frequently
referring people to it. It's written by a Google engineer who does a much better
job of articulating good architecture techniques than I could. A must-read for
any software developer: [Writing Testable Code by Misko Hevery][1]

This next post is something I just came across, but it does a really good job
explaining why writing testable code is not just about validating your code's
functionality. Because it's something I run into a lot in dynamic languages like
Python or Qt/C++, I would also add that hacky workarounds (like Python's
`unittest.mock.patch()`) exist that let you technically validate your code
without actually writing what is considered to be well-designed, testable code.
Without furhter ado: [Write testable code even if you don't write tests][2] by
Karl Seguin.

[1]: http://googletesting.blogspot.com/2008/08/by-miko-hevery-so-you-decided-to.html
[2]: http://openmymind.net/2010/8/17/Write-testable-code-even-if-you-dont-write-tests/
