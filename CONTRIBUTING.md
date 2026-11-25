# Contributing

Thank you for helping. The project needs three kinds of contributions most.

## Device sessions and dumps

Facts in the specification are marked *likely* until a real device confirms them. A
recorded download from your dive computer can turn a whole family document into
*verified* facts and becomes a regression test for the Rust code.

Until the recorder ships, open an issue with your device model, firmware version and the
transport you use (cable, Bluetooth, infrared). Recordings contain your dive data; share
only what you are comfortable publishing.

## Specification corrections

Edit the Markdown under `docs/spec/`. Follow `docs/spec/conventions.md` and
`docs/spec/TEMPLATE.md`, state the source of the fact in the pull request description,
and set the confidence level accordingly.

## Code

Read [ARCHITECTURE.md](ARCHITECTURE.md) and the Rust rules in [AGENTS.md](AGENTS.md),
which apply to human contributors too.

## Clean-room rule

This project is MIT OR Apache-2.0 and must stay free of GPL and LGPL code. You may learn
facts from any source, including other open-source dive-log software, but never copy
code, comments, identifiers or prose from it. Describe and implement everything in your
own words, from the specification.

## Checks

```bash
mdbook build docs
lychee --offline --include-fragments --exclude-path docs/SUMMARY.md 'docs/**/*.md'
```

Pull request titles follow [Conventional Commits](https://www.conventionalcommits.org/),
for example `docs(shearwater): correct closing record layout`.
