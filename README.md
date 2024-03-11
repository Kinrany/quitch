# quitch

An attempt to reimplement parts of sqitch

> Elymus repens, also known as couch grass or quitch, is a very common species
> of grass native to most of Europe, Asia, the Arctic biome, and northwest
> Africa. It has been brought into other mild northern climates for forage or
> erosion control, but is often considered a weed.

## Examples

```bash
git clone https://github.com/Kinrany/quitch.git
cd quitch
deno task quitch revert --target mysql://user:pass@localhost:3306/db --plan-file ../db/sqitch.plan
```
