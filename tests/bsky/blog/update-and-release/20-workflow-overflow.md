+++
title = "Overview of Our Workflow"
description = "We’ll kick things off with a detailed overview of the dependency update and release workflow. We’ll cover the key steps involved, from identification of updates through automated testing and release."
date = 2025-01-17
updated = 2025-01-16
draft = false

[taxonomies]
topic = ["Technology"]
tags = ["devsecops", "software", "circleci", "security", "practices"]

[extra]
bluesky = "Covering the key steps involved, from identification of updates through automated testing and release."

+++
<!-- markdownlint-disable MD003 MD024 MD033-->

## Inputs to a weekly release

{% mermaid() %}
---

config:
  look: handDrawn
  theme: forest
---

flowchart LR
    C1@{ shape: sm-circ, label: "Start" }
    P10@{ shape: rect, label: "Development" }
    I10@{ shape: docs, label: "Development PRs" }
    P20@{ shape: rect, label: "Dependency <br> Checks" }
    I20@{ shape: docs, label: "Dependency PRs" }
    P30@{ shape: rect, label: "CI Test<br> & Review  " }
    I30@{ shape: doc, label: "Updated Repo" }
    P40@{ shape: rect, label: "Release" }
    I40@{ shape: docs, label: "Published Release" }
    C2@{ shape: framed-circle, label: "Stop" }
    C1 --> P10 --> I10 --> P30 --> I30 --> P40 --> I40 --> C2
    C1 --> P20 --> I20 --> P30
{% end %}

The diagram describes the processes, outputs and inputs through the release process that contribute to building and, ultimately, releasing a new version of the software.

The development and dependency check processes create pull requests to facilitate the CI testing and review. Once passed, the changes are merged into the main branch.

CircleCI triggers the release process weekly on the main branch, updates the version numbers, and publishes the release.

## Schedule of weekly workflow

{% mermaid() %}
---

config:
  look: handDrawn
  theme: forest
  displayMode: compact
---

gantt
    title Update and Release Cycle
    dateFormat YYYY-MM-DD
    tickInterval 1day
    todayMarker off
    axisFormat %d
section Dev
        Cycle starts :active,  milestone, m1, 2025-01-01, 2m
        Development         :active, dev, 2025-01-01, 5d
section Dep
        Dependency Updates  :active, dep, 2025-01-05, 1d
section Pub
        Release             :active, after dev and dep, 1d
        Risk                :                           1d
        Cycle Ends : active,  milestone, m2, 2025-01-08, 4m

{% end %}

The gantt chart shows the schedule of the release cycle. The development phase starts on day 1 on the cycle and ends on day 5. The dependency updates are scheduled for day 5 and the release workflow runs on day 6.

The final day of the seven-day cycle is for rest (or remediation is required).

For most of the libraries and tools under development day one starts at the beginning of the week on Monday. However, some weeks are shifted to account for dependencies, particularly the packaging of the docker container for CI which is released on Tuesdays to update with the tools as release in the previous week's work.
