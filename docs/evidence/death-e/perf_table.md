# DEATH-E1 probe tables

The GPU column is not a usable figure (shared machine, see the evidence README); the GPU cost of the deaths was not measured.

Frame interval ms by phase of the pileup (mean / max, median of runs):

| Resolution | Scenario | impact 71-160 | peak 161-260 | settle 261+ |
| --- | --- | --- | --- | --- |
| 640x360 | baseline (nobody dies) | 16.65 / 25.84 | 16.65 / 28.48 | 16.67 / 33.77 |
| 640x360 | pileup, preset 1 | 16.68 / 21.92 | 16.67 / 25.70 | 16.67 / 44.62 |
| 640x360 | pileup, preset 4 | 16.69 / 21.62 | 16.66 / 55.22 | 16.70 / 54.68 |
| 640x360 | pileup + stress, preset 1 | 16.71 / 54.17 | 16.65 / 61.03 | 16.69 / 42.97 |
| 640x360 | pileup + stress, preset 4 | 18.44 / 82.84 | 16.44 / 30.43 | 16.71 / 48.34 |
| 1920x1080 | baseline (nobody dies) | 68.48 / 118.28 | 76.30 / 148.99 | 75.43 / 353.47 |
| 1920x1080 | pileup, preset 1 | 66.80 / 150.94 | 66.22 / 107.41 | 63.44 / 116.70 |
| 1920x1080 | pileup, preset 4 | 69.78 / 94.40 | 71.89 / 122.94 | 77.25 / 201.81 |
| 1920x1080 | pileup + stress, preset 1 | 73.31 / 138.63 | 62.53 / 152.40 | 70.46 / 111.86 |
| 1920x1080 | pileup + stress, preset 4 | 67.92 / 131.80 | 69.49 / 161.60 | 68.66 / 170.70 |

All probes:

| Resolution | Scenario | frame ms mean / p95 / max | GPU ms mean / p95 / max (not usable, see README) | tick us mean / max | sprite sync us mean / max | RSS MiB (peak) | entities | peak parts / floor / wall / visible sprites | final recycled (parts, floor, wall) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 640x360 | baseline (nobody dies) | 16.66 / 18.70 / 33.77 | 0.180 / 0.187 / 0.751 | 9.6 / 58 | 8.8 / 67 | 420 (421) | 4092 | 0 / 0 / 0 / 0 | 0, 0, 0 |
| 640x360 | pileup, preset 1 | 16.67 / 19.32 / 44.62 | 0.642 / 1.040 / 3.775 | 14.0 / 107 | 12.0 / 78 | 383 (388) | 4110 | 4 / 0 / 0 / 8 | 0, 0, 0 |
| 640x360 | pileup, preset 4 | 16.69 / 19.80 / 55.22 | 0.707 / 1.388 / 4.508 | 21.9 / 2162 | 20.3 / 768 | 385 (388) | 4110 | 128 / 130 / 18 / 238 | 0, 0, 0 |
| 640x360 | pileup + stress, preset 1 | 16.98 / 21.77 / 61.03 | 0.424 / 0.827 / 2.159 | 36.7 / 540 | 20.6 / 483 | 386 (388) | 4110 | 227 / 0 / 0 / 231 | 0, 0, 0 |
| 640x360 | pileup + stress, preset 4 | 16.93 / 20.97 / 82.84 | 0.466 / 1.016 / 2.229 | 95.4 / 2555 | 38.1 / 209 | 387 (391) | 4110 | 256 / 192 / 32 / 730 | 1814, 2661, 442 |
| 1920x1080 | baseline (nobody dies) | 74.35 / 103.57 / 353.47 | 2.478 / 4.523 / 8.245 | 15.5 / 78 | 11.4 / 68 | 394 (396) | 4092 | 0 / 0 / 0 / 0 | 0, 0, 0 |
| 1920x1080 | pileup, preset 1 | 63.93 / 77.20 / 150.94 | 1.169 / 2.452 / 4.317 | 19.2 / 159 | 14.0 / 77 | 398 (435) | 4110 | 4 / 0 / 0 / 8 | 0, 0, 0 |
| 1920x1080 | pileup, preset 4 | 75.76 / 93.56 / 201.81 | 1.676 / 5.299 / 7.906 | 24.0 / 152 | 24.2 / 386 | 401 (435) | 4110 | 128 / 130 / 18 / 238 | 0, 0, 0 |
| 1920x1080 | pileup + stress, preset 1 | 68.43 / 88.59 / 152.40 | 2.007 / 3.660 / 6.729 | 30.1 / 165 | 18.2 / 131 | 399 (400) | 4110 | 227 / 0 / 0 / 231 | 0, 0, 0 |
| 1920x1080 | pileup + stress, preset 4 | 68.56 / 95.18 / 170.70 | 2.097 / 3.540 / 7.848 | 87.3 / 1812 | 32.7 / 213 | 397 (398) | 4110 | 256 / 192 / 32 / 730 | 1814, 2661, 442 |
