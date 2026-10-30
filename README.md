# in rust we t(rust) - AP 2026 Project
This repository contains **our group's project contributions** for the Advanced Programming course of the academic year 2026/2027.

## Members and contributions
This is the schema defining the group and each member's contributions to the various components:
| Member | Contributions |
|---|---|
| Thomas Moretti | TBD |
| Rocco Daoglio | TBD |
| Silvia Ianni | TBD |
| Flavio Savianu | TBD |

## Development
This repository implements the following components:
  - The chart (`crates/chart`)
  - The harness (`crates/game`)
  - The admiralty (`crates/admiralty`)
  - The cartographer (`crates/cartographer`)
  - The islands (`crates/island`)
  - The ship (`crates/ship`)

## Continuous Integration
On push to any branch, and on pull requests to `dev` and `main`, a CI workflow is ran. What it does is, essentially, checking the formatting, running `clippy` and also running the unit tests.
