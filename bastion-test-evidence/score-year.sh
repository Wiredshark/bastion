#!/bin/bash
# ITEM 24 bar 2 scorer — the annual cycle, unpinned.
# BAR: food stock rises through Spring/Summer, falls through Winter.
# tick→day: day_length=0.5 min ⇒ 900 ticks/day (30 tps × 30 s).
# Seasons (days_in_year=160): Spring 0-39, Summer 40-79, Autumn 80-119,
# Winter 120-159 (repeat mod 160).
EV="${PIT_EV:-/e/veloren-master/.engine-integration-wt/bastion-test-evidence}"
S=$(sed 's/\x1b\[[0-9;]*m//g' "$EV/server-pit-year.log" 2>/dev/null)

echo "=== PRECONDITIONS ==="
grep -E "day_length delivered" "$EV/pit-year.log" 2>/dev/null | head -1
echo "seed emit: $(echo "$S" | grep -c 'SEED_FOOD active')"
echo "samples  : $(echo "$S" | grep -c 'food stock sample')"
echo "stage-ups by season (the growth engine's own witness):"
echo "$S" | grep "crop stage-up" | grep -oE "season=[A-Za-z]+" | sort | uniq -c

echo "=== THE SERIES (per-season food-stock mean; day = tick/900 mod 160) ==="
echo "$S" | grep "food stock sample" | grep -oE "tick=[0-9]+ food_stock=[0-9]+" \
  | awk '{split($1,t,"="); split($2,f,"=");
          day=(t[2]/900)%160;
          s=(day<40)?"1-Spring":(day<80)?"2-Summer":(day<120)?"3-Autumn":"4-Winter";
          sum[s]+=f[2]; n[s]++}
         END{for (k in sum) printf "%s: mean=%.1f n=%d\n", k, sum[k]/n[k], n[k]}' \
  | sort
echo "=== first/last 5 samples (raw trend endpoints) ==="
echo "$S" | grep "food stock sample" | grep -oE "tick=[0-9]+ food_stock=[0-9]+" | head -5
echo "$S" | grep "food stock sample" | grep -oE "tick=[0-9]+ food_stock=[0-9]+" | tail -5
