---
Title: >
    5 myths about infrastructure-as-code via general purpose programming
    languages
Date: 2020-04-15
---

There's an ongoing debate among infrastructure-as-code practitioners between
configuration languages like YAML and HCL and using "real programming
languages" (including domain specific programming languages) to generate
configuration. The debate is going very poorly because there is a lot
of confusion about the "real programming languages" position, so I want to
correct some of the common points of confusion ("myths") so the conversation
can focus on more substantial concerns.

<!-- more -->

# Disclaimer

These are all concerns I've heard/read. They aren't strawman arguments, and
anyway popular misconceptions among my 'opponents' doesn't imply that their
conclusion is wrong or that mine is right (all of these myths could be
widely believed and yet there could still be--and certainly are--good arguments
for configuration languages). Further, I'm sure not everyone in the
configuration language camp believes any or all of these myths, and many
people that I respect are in the configuration language camp (for now,
anyway!).

# Myths

## 1. Using a programming language implies imperatively updating infrastructure

Some people are concerned that an infinite loop in their program might spawn
an infinite number of VMs or other resources. Others imagine that they will
have to reimplement much of Terraform/CloudFormation/etc including logic to
determine what updates need to be applied, how to rollback, deletion
protection, etc.

The key misunderstanding is that "generating configuration" means exactly
that--you're generating the static configuration that is then fed into a tool
like Terraform or CloudFormation. The tools still do the hard part: figuring
out what updates to apply, rollbacks, etc.

## 2. Using a programming language introduces unpredictability

Some are worried that if they aren't reading and writing static configuration
files and committing them to git, it will be opaque to them what changes will
be applied. This is probably an extension of the aforementioned confusion
between "imperatively updating infrastructure" and "generating
configuration". Importantly, because our scripts' output is itself input to an
infrastructure-as-code solution, it's always possible to inspect that directly
just like if your team had written it directly, modulo perhaps comments, style,
etc. Further, Terraform and CloudFormation also have the ability to preview
changes before you apply them, and those same features are also available.

## 3. Using a programming language is incompatible with declarative infrastructure

I think this is just a misunderstanding of what "declarative" means. I think
people initially confuse it with "static" or "constant" (as in "it doesn't
execute or evaluate", not as in "statically typed"). This is odd because
CloudFormation and Terraform's respective configuration languages aren't
strictly static in any meaningful way--they describe (or "declare")
instructions that must be evaluated by CF/TF; just because the AST is expressed
via YAML or HCL instead of having semantics expressed in syntax doesn't make
it meaningfully static. This is implied by CloudFormation's "template" noun
(templates are evaluated by definition).

Declarative also doesn't mean "static", it means (something like) the
composition of evaluatable expressions ("static configuration" being a strict
subset thereof), and there are many programming languages that meet this
criteria and many more that allow for a declarative style. For example, you
can use a very declarative subset of Python to evaluate into YAML:

```py
# This is an example from a prototype Python-to-CloudFormation library I wrote:
# https://github.com/weberc2/nimbus/blob/46f464fdaa53a4c97f5df1db87bf8f175ae8db1c/examples/src/nimbus_examples/s3bucket.py
bucket_name_parameter = ParameterString(Description="The name of the bucket")
key_arn_parameter = ParameterString(
    Description="The ARN of the KMS key used to encrypt the bucket",
)
bucket = Bucket(BucketName=bucket_name_parameter)
t = Template(
    description="S3 Bucket Template",
    parameters={
        "BucketName": bucket_name_parameter,
        "KMSKeyARN": key_arn_parameter,
    },
    resources={
        "Bucket": bucket,
        "BucketPolicy": ManagedPolicy(
            PolicyDocument={
                "Version": "2012-10-17",
                "Statement": [
                    {
                        "Sid": "AllowFullAccessToBucket",
                        "Action": "s3:*",
                        "Effect": "Allow",
                        "Resource": Sub(
                            f"${{BucketARN}}/*",
                            BucketARN=bucket.GetArn()
                        ),
                    },
                    {
                        "Sid": "AllowUseOfTheKey",
                        "Effect": "Allow",
                        "Action": [
                            "kms:Encrypt",
                            "kms:Decrypt",
                            "kms:ReEncrypt*",
                            "kms:GenerateDataKey*",
                            "kms:DescribeKey",
                        ],
                        "Resource": key_arn_parameter,
                    },
                    {
                        "Sid": "AllowAttachmentOfPersistentResources",
                        "Effect": "Allow",
                        "Action": [
                            "kms:CreateGrant",
                            "kms:ListGrants",
                            "kms:RevokeGrant",
                        ],
                        "Resource": key_arn_parameter,
                        "Condition": {
                            "Bool": {"kms:GrantIsForAWSResource": True}
                        },
                    },
                ],
            },
        ),
    },
)

print(yaml.dump(t.template_to_cloudformation()))
```

## 4. Programming languages are necessarily turing complete (and that's a big problem)

There's a potential semantic argument about whether or not 'programming
language' implies 'Turing completeness'. Certainly most are, and I'm happy to
concede that argument; the thing I care about is that the language can evaluate
a fixed set of inputs to a (set of?) arbitrary CloudFormation template(s) or
Terraform module(s). This definition of "programming language" includes things
like [Starlark](https://go.starlark.net) and [Dhall](https://dhall-lang.org/)
are valid solutions (Starlark looks like Python though not as flexible and
it's an easier sell to developers; however, there is no static analysis
tooling for it. By contrast Dhall is built by Haskellers so no one wants to
learn it but it has a static type system) even though they aren't Turing
complete.

One concern that has been expressed to me is that a Turing complete language
would allow someone ('s coworkers) to spin up infinite VMs due to an accidental
infinite loop. First of all, if you use a declarative language or style of
programming, you aren't writing unbounded loops. Secondly, infinite loops and
other non-terminating bugs are a rare class of bug in general and especially
the kinds of languages that are amenable to generating YAML/etc. Thirdly, if
one of these infinite loops was introduced, it would merely fail to generate
the configuration to pass into Terraform/CloudFormation; it wouldn't try to
stand up infrastructure in the loop.

Rather than being concerned about Turing completeness, I would be more worried
that someone would write code with side effects--accessing disk or
network--however, that sort of bug seems very unlikely and easy enough to catch
in code review (and if your organization really can't be trusted to
consistently avoid these bugs, then any embedded scripting language would
suit, including Dhall and Starlark).

## 5. The principle of least power says that we should use configuration languages

The principle of least power states that we should aim for "the least powerful
language that is *suitable for the task*" (emphasis mine) not the least
powerful language (period). The question is whether or not YAML/HCL are
suitable for the task, and I think they are not because everything seems to
be trending in the direction of general purpose languages:

* Terraform and CloudFormation's featuresets are becoming increasingly dynamic
  / powerful with each release
* AWS recently released a CDK to allow developers to declare their
  infrastructure with popular programming languages
* Pulumi also allows developers to declare infra with popular programming
  languages

