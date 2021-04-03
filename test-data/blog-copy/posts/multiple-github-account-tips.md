---
Title: Tips for working with multiple GitHub accounts
Date: 2020-04-25
---

I use GitHub for my work and personal projects with different profiles for
each. Because it's a good security/privacy practice, each profile has its
own distinct SSH key. However, this causes problems because the `git` CLI
will always try to use the first SSH key that maps to the `github.com`
domain even if that key has no permissions for the target repository. The
other more straightforward problem with multiple accounts is that the
GitHub.com browser cookie asserts that you are only logged into one account
at a time.

My solutions for these problems are [`direnv`][0] and [Firefox
Containers][1], respectively. These use cases are straightforward
applications of these technologies, so I'm not claiming any innovation here,
but rather it took me a long time to identify these solutions, and I hope
this saves others some time. If you're not familiar with these tools, read
on for details.

<!-- more -->

<!-- h3 because the site header is h1 and the post title is h2 -->
# Solution 1: Managing multiple GitHub SSH keys with [`direnv`][0]

The `git` CLI respects a `GIT_SSH_COMMAND` environment variable, and the `ssh`
CLI takes a `-i` flag to specify an "identity file" or private key. If I run
`GIT_SSH_COMMAND=$HOME/.ssh/github-personal git push`, it will try to push the
current repo using my personal github token. However, I don't want to have to
manage setting and unsetting that variable as I switch between work and
personal repositories; I want that setting to "stick" to each repository.

By-and-large, [`direnv`][0] serves this purpose. It looks for `.envrc` files
in the shell's current working directory and its ancestor directories and
sources them automatically as the shell's working directory changes or as the
relevant `.envrc` files are changed. It's widely supported and very easy to
install (`brew install direnv`) and configure:

```bash
# Make sure to add .envrc to your .gitignore!
cd $PERSONAL_REPO && \
    echo 'GIT_SSH_COMMAND="ssh -i $HOME/.ssh/github-personal"' >> .envrc && \
    direnv allow`
```

## Limitation

This doesn't work if you're using `git -C` to run commands on a git repository
outside that is not an ancestor of your current working directory, e.g., if you
are in `$HOME` and your personal repo is `$HOME/personal` with a direnv envrc
file at `$HOME/personal/.envrc`, running `git -C $HOME/personal push` will not
trigger `direnv` to load your `.envrc` file because `direnv` hooks into your
shell, not into the `git` CLI. To support the `git -C` usecase, you'll need to
pass the `GIT_SSH_COMMAND` env var to the `git` subprocess.

# Solution 1 Alternative: the SSH config trick

For a while, I was using the [SSH config trick][2] which told `ssh` to use my
personal SSH key for requests to the host `github-personal` while using my work
key as the default for requests to `github.com`. This required me to change my
git config for my private repos to use the `github-personal` host. This worked
reasonably well until I wanted to write scripts that worked on my MacBook as
well as in a CI environment--I didn't want to parameterize the host because
that's a weird thing to do--it's always going to GitHub, and any tool that
wants to make authenticated requests to GitHub on my behalf would also have to
support this kind of host parameterization.

# Solution 2: Using Firefox Containers for multiple accounts

Firefox has a feature called "Containers" which are basically collections of
tabs that share the same cookies, history, etc. Each container is sort of its
own browser, in a sense--containers are isolated from each other, as the name
implies. So I when I log into GitHub using my personal account in the
"Personal" container, I can simultaneously be logged into my work GitHub
account on the "Work" container (or rather, the default container, as
appropriate) because the two containers don't share cookie jars. When I open a
new Personal container tab and navigate to github.com, I'm already signed into
my personal GH account, and vice versa for my work account in Work container
tabs.

# Conclusion

I use these tools for lots of other applications as well, including AWS
(including using `direnv` to set `AWS_DEFAULT_PROFILE` and
`AWS_DEFAULT_REGION` for the `aws` CLI) and the Google suite.

If you have other solutions that you've used for similar problems, or feedback
or other suggestions, I'd love to hear them. Reach out via
[Twitter](https://twitter.com/weberc2) or email (weberc2 / gmail).


[0]: https://direnv.net/
[1]: https://support.mozilla.org/en-US/kb/containers
[2]: https://gist.github.com/oanhnn/80a89405ab9023894df7
