# quitch

An attempt to reimplement parts of sqitch

> Elymus repens, also known as couch grass or quitch, is a very common species
> of grass native to most of Europe, Asia, the Arctic biome, and northwest
> Africa. It has been brought into other mild northern climates for forage or
> erosion control, but is often considered a weed.

## Install

Two implementations are available. The one in Rust is new and more promising,
the one in Deno has one more command but will probably be removed eventually.

### Rust

1. [Install Rust](https://rustup.rs/)
2. `cargo install quitch`

### Deno

1. [Install Deno](https://docs.deno.com/runtime/manual)
2. `deno install --global --allow-read --allow-net https://deno.land/x/quitch@v0.0.4/main.ts`
3. Run `export PATH="$HOME/.deno/bin:$PATH"` to make available in the current
   shell
4. Add `export PATH="$HOME/.deno/bin:$PATH"` to your `~/.bashrc` or `~/.zshrc`

## Use

```bash
# Revert the last change
quitch revert --target mysql://user:pass@localhost:3306/db --plan-file ../some-db/sqitch.plan
```
