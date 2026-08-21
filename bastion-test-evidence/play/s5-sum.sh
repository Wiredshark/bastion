#!/bin/bash
# Summarise one turn log into trend numbers. Reads the log the driver wrote.
L=$1
sed 's/\x1b\[[0-9;]*m//g' "$L" | awk '
/waited .* ticks/ {print "  " $0}
/COLONY tick=/ {print "  " substr($0, index($0,"COLONY"))}
/INSPECT uid=/ {
  n++
  for(i=1;i<=NF;i++){
    split($i,a,"=")
    if(a[1]=="hunger") h+=a[2]
    if(a[1]=="rest") r+=a[2]
    if(a[1]=="energy") e+=a[2]
    if(a[1]=="mood") m+=a[2]
    if(a[1]=="health"){ gsub(/Some\(|\)/,"",a[2]); if(a[2]=="None"){hn++} else {hp+=a[2]; hc++} }
    if(a[1]=="drive") d[a[2]]++
    if(a[1]=="activity"){ if(a[2]=="None") idle++; else act++ }
  }
}
/inspect_colonists ->/ {
  printf "  N=%d meanHunger=%.4f meanRest=%.4f meanEnergy=%.4f meanMood=%.4f meanHealth=%.4f healthNone=%d activityNone=%d\n", n, h/n, r/n, e/n, m/n, (hc?hp/hc:-1), hn+0, idle+0
  printf "  drives:"; for(k in d) printf " %s=%d", k, d[k]; printf "\n"
  n=0;h=0;r=0;e=0;m=0;hp=0;hc=0;hn=0;idle=0;act=0; delete d
}
/ITEMS count=/ {print "  " substr($0, index($0,"ITEMS count=")) }
'
