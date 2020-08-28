# jetson-cool

## Scripts & Tools

`jetson_clocks`: script that disables the `DVFS` governor and locks the clocks to their maximums as defined by the active `nvpmodel` power mode. If the active `nvpmodel` mode is 10W, `jetson_clocks` will lock the clocks to their maximums for 10W mode. If the active `nvpmodel` is 5W, `jetson_clocks` will lock the clocks to their maximums for 5W mode.

`/etc/nvpmodel.conf` defines the `nvpmodel`s.

## Code

[procfs](https://docs.rs/procfs/0.8.0/) crate.

## Acronyms

`nvpmodel`: `NV`idia `P`ower `MODEL`

`DVFS`: `D`ynamic `V`oltage and `F`requency `S`caling 
