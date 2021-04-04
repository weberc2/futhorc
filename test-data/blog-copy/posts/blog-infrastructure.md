---
Title: New blog infrastructure
Date: 2019-04-06
Tags: [meta]
---

I *finally* got around to automating the publishing of this blog. It hasn't
been a high priority, since I only post a couple of times a year, but it's
always bothered me that something that is such an ideal candidate for
automation hasn't been automated. Anyway, I finally did it and I want to
describe the setup in case it's helpful for anyone looking to do the same.

<!-- more -->

<!--
NOTE: headers start at h3 b/c h1 is for site header and h2 is for post title
-->

# Context

The blog is published to Bitbucket pages (I used Bitbucket because it supports
Mercurial and life is too short to use unpleasant tools in your hobby time),
and I wrote [my own static-site generator][0] (this is not an advertisement; do
not use it; it's not remotely "production ready") because at least at the time
Hugo seemed overly complex (and thus tedious to learn) for my use-case and
writing my own seemed like more fun.

I decided to put my sources in one repository and the generated output in the
repository which would be published. The target repo is called
weberc2.bitbucket.io--this naming convention is (or at least it was when I
started my blog) the mechanism by which Bitbucket knew to serve your blog as a
static site. The source repo is a private repo.

# Old Workflow

The workflow was to write a post in the source repo's `posts/` directory, pull
down the latest changes to the target repo, clear out the target repo, run the
generator (copying output files into the empty target repo), and finally
commit/push the changes to both repos.

# Automation Workflow

Instead, I wanted to be able to work against the source repo and have it
automatically publish when I pushed changes. To do that, I updated my static
site generator with a Bitbucket Pipeline (a CI pipeline tool integrated into
Bitbucket) to build and publish Docker images.

```yaml
pipelines:
  default:
    - step:
        services:
          - docker
        caches:
          - docker
        script:
          - docker build -t weberc2/neon:latest .
          - docker login --username $DOCKER_USERNAME --password $DOCKER_PASSWORD
          - docker tag weberc2/neon weberc2/neon:$BITBUCKET_COMMIT
          - docker push weberc2/neon:latest
          - docker push weberc2/neon:$BITBUCKET_COMMIT
```

I also created a pipeline in the source repository which uses that Docker image
to run the static site generator and update the target repository.

```yaml
pipelines:
  default:
    - step:
        services:
          - docker
        caches:
          - docker
        script:
          # NOTE that we're deliberately not passing `-it`; passing it yields
          # the error: "the input device is not a TTY"
          - docker run --rm -v $PWD:/site --workdir /site weberc2/neon neon build
          - hg clone ssh://hg@bitbucket.org/weberc2/weberc2.bitbucket.org /weberc2.bitbucket.org
          - rm -rf /weberc2.bitbucket.org/*
          - cp -r ./_output/* /weberc2.bitbucket.org/
          - cd /weberc2.bitbucket.org
          - hg addremove
          - hg commit -m "Automatic update from source.weberc2.bitbucket.org @ $BITBUCKET_COMMIT"
          - hg push
```

This took some work to allow the source repo's pipeline to write to the target
repo--basically I created an SSH key for the source pipeline and added its
public key to my Bitbucket profile so the target repository would allow the
pipeline to update it as though I were making the updates. This isn't ideal,
since I'm giving the pipeline access to my whole repo, but the alternative is
to create a Bitbucket user for each pipeline (which I may eventually do).
Ideally the target repo would allow me to assign permissions for the pubkey
without having to map the pubkey to a Bitbucket user, but alas...

It only took me an hour or so to build this out. If you're interested in doing
something similar and/or have questions about my setup. Feel free to reach out
to me on [Twitter][1].

# Pipelines recap

I want to take a moment to plug [Bitbucket Pipelines][2]. The user experience
is nothing short of fantastic. The only issue I ran into was the permissioning
bit, but that's hardly insurmountable. The docs are excellent, the secrets
management works as expected, the UI is intuitive, etc. I've been steeped in
frustrating CI tools for the last 6 months, and Pipelines really stands out as
a solid tool. Definitely the best thing I've found for getting a small project
up and running quickly. I'm not getting any kickbacks for this paragraph; just
giving credit where it's due.

[0]: https://bitbucket.org/weberc2/neon
[1]: https://twitter.com/weberc2
[2]: https://www.bitbucket.org/product/features/pipelines
