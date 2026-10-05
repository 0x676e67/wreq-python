# WSL blocking rerun review

Observed per-cell differences against the selected blocking snapshot. These runs were collected in separate sessions with different source/native builds; ratios do not isolate any one code change. The new ST variant has no historical blocking baseline.

Before: `80b48db8b2741b60b41ba6f0966bd5f52c3f5185` (2026-10-04T13:39:55.943939+00:00).
After: `82b372d0006a5ef71e82b1bcde9df80acba01bb0` (2026-10-05T09:01:56.555288+00:00).

## Common-client summary

| Client | Cells | Higher/lower | Geometric mean ratio | Median new CV (%) |
| --- | ---: | --- | ---: | ---: |
| wreq (blocking MT) | 112 | 99/13 | 1.0305 | 1.14 |
| ry (blocking) | 112 | 109/3 | 1.0364 | 0.85 |
| requests | 56 | 56/0 | 1.0360 | 0.64 |
| httpx (blocking) | 112 | 110/2 | 1.0278 | 0.50 |
| niquests (blocking) | 112 | 109/3 | 1.0327 | 0.57 |
| curl_cffi (blocking) | 112 | 112/0 | 1.0244 | 0.38 |
| PycURL | 112 | 110/2 | 1.0315 | 0.65 |

## wreq ST versus MT in this run

| Protocol | Upload | Payload | Concurrency | MT RPS | ST RPS | ST/MT |
| --- | --- | --- | ---: | ---: | ---: | ---: |
| h1 | full | 1 KiB | 10 | 19830.18 | 21025.74 | 1.0603 |
| h1 | full | 1 KiB | 50 | 15894.76 | 17266.47 | 1.0863 |
| h1 | full | 1 KiB | 100 | 13270.06 | 14605.17 | 1.1006 |
| h1 | full | 1 KiB | 150 | 11482.57 | 12539.17 | 1.0920 |
| h1 | full | 10 KiB | 10 | 19323.55 | 20410.84 | 1.0563 |
| h1 | full | 10 KiB | 50 | 15960.32 | 17449.37 | 1.0933 |
| h1 | full | 10 KiB | 100 | 13033.26 | 14487.35 | 1.1116 |
| h1 | full | 10 KiB | 150 | 11442.41 | 12397.87 | 1.0835 |
| h1 | full | 64 KiB | 10 | 18691.14 | 15560.46 | 0.8325 |
| h1 | full | 64 KiB | 50 | 14933.17 | 14967.96 | 1.0023 |
| h1 | full | 64 KiB | 100 | 12640.69 | 13538.50 | 1.0710 |
| h1 | full | 64 KiB | 150 | 10995.99 | 11982.66 | 1.0897 |
| h1 | full | 128 KiB | 10 | 17637.09 | 11697.90 | 0.6633 |
| h1 | full | 128 KiB | 50 | 14479.16 | 9799.95 | 0.6768 |
| h1 | full | 128 KiB | 100 | 12134.56 | 9014.67 | 0.7429 |
| h1 | full | 128 KiB | 150 | 10090.43 | 7887.64 | 0.7817 |
| h1 | full | 1 MiB | 10 | 4987.94 | 1522.33 | 0.3052 |
| h1 | full | 1 MiB | 50 | 4359.47 | 1479.35 | 0.3393 |
| h1 | full | 1 MiB | 100 | 4040.45 | 1478.27 | 0.3659 |
| h1 | full | 1 MiB | 150 | 3895.27 | 1514.44 | 0.3888 |
| h1 | full | 2 MiB | 10 | 2240.33 | 894.02 | 0.3991 |
| h1 | full | 2 MiB | 50 | 2069.77 | 863.48 | 0.4172 |
| h1 | full | 2 MiB | 100 | 1938.86 | 857.52 | 0.4423 |
| h1 | full | 2 MiB | 150 | 1852.60 | 854.43 | 0.4612 |
| h1 | full | 4 MiB | 10 | 1023.24 | 442.73 | 0.4327 |
| h1 | full | 4 MiB | 50 | 995.50 | 465.31 | 0.4674 |
| h1 | full | 4 MiB | 100 | 954.77 | 457.41 | 0.4791 |
| h1 | full | 4 MiB | 150 | 883.57 | 458.01 | 0.5184 |
| h1 | stream | 1 KiB | 10 | 8128.62 | 8196.89 | 1.0084 |
| h1 | stream | 1 KiB | 50 | 7156.76 | 7556.34 | 1.0558 |
| h1 | stream | 1 KiB | 100 | 6681.92 | 7195.08 | 1.0768 |
| h1 | stream | 1 KiB | 150 | 6287.02 | 6665.73 | 1.0602 |
| h1 | stream | 10 KiB | 10 | 8178.71 | 7837.64 | 0.9583 |
| h1 | stream | 10 KiB | 50 | 7190.70 | 7355.54 | 1.0229 |
| h1 | stream | 10 KiB | 100 | 6733.02 | 6886.37 | 1.0228 |
| h1 | stream | 10 KiB | 150 | 6192.03 | 6545.45 | 1.0571 |
| h1 | stream | 64 KiB | 10 | 4396.31 | 3960.05 | 0.9008 |
| h1 | stream | 64 KiB | 50 | 4120.12 | 3800.06 | 0.9223 |
| h1 | stream | 64 KiB | 100 | 3883.44 | 3594.94 | 0.9257 |
| h1 | stream | 64 KiB | 150 | 3697.82 | 3513.83 | 0.9502 |
| h1 | stream | 128 KiB | 10 | 4343.64 | 3449.93 | 0.7942 |
| h1 | stream | 128 KiB | 50 | 4038.40 | 3277.33 | 0.8115 |
| h1 | stream | 128 KiB | 100 | 3854.86 | 3052.42 | 0.7918 |
| h1 | stream | 128 KiB | 150 | 3637.20 | 2907.50 | 0.7994 |
| h1 | stream | 1 MiB | 10 | 1427.21 | 750.92 | 0.5261 |
| h1 | stream | 1 MiB | 50 | 1386.86 | 728.76 | 0.5255 |
| h1 | stream | 1 MiB | 100 | 1334.69 | 696.53 | 0.5219 |
| h1 | stream | 1 MiB | 150 | 1319.64 | 685.95 | 0.5198 |
| h1 | stream | 2 MiB | 10 | 1279.74 | 475.38 | 0.3715 |
| h1 | stream | 2 MiB | 50 | 1265.62 | 463.04 | 0.3659 |
| h1 | stream | 2 MiB | 100 | 1199.23 | 450.46 | 0.3756 |
| h1 | stream | 2 MiB | 150 | 1177.09 | 444.20 | 0.3774 |
| h1 | stream | 4 MiB | 10 | 947.88 | 303.99 | 0.3207 |
| h1 | stream | 4 MiB | 50 | 951.77 | 278.22 | 0.2923 |
| h1 | stream | 4 MiB | 100 | 881.18 | 273.65 | 0.3105 |
| h1 | stream | 4 MiB | 150 | 872.29 | 278.54 | 0.3193 |
| h2 | full | 1 KiB | 10 | 19206.81 | 19146.59 | 0.9969 |
| h2 | full | 1 KiB | 50 | 15566.42 | 16743.37 | 1.0756 |
| h2 | full | 1 KiB | 100 | 13215.84 | 14518.41 | 1.0986 |
| h2 | full | 1 KiB | 150 | 11335.87 | 12438.61 | 1.0973 |
| h2 | full | 10 KiB | 10 | 19127.76 | 17240.01 | 0.9013 |
| h2 | full | 10 KiB | 50 | 15439.76 | 15691.89 | 1.0163 |
| h2 | full | 10 KiB | 100 | 13039.35 | 14474.28 | 1.1100 |
| h2 | full | 10 KiB | 150 | 11341.20 | 12436.36 | 1.0966 |
| h2 | full | 64 KiB | 10 | 11867.41 | 7285.22 | 0.6139 |
| h2 | full | 64 KiB | 50 | 10195.32 | 7340.82 | 0.7200 |
| h2 | full | 64 KiB | 100 | 9162.33 | 6778.00 | 0.7398 |
| h2 | full | 64 KiB | 150 | 8306.01 | 6680.64 | 0.8043 |
| h2 | full | 128 KiB | 10 | 10963.26 | 5259.90 | 0.4798 |
| h2 | full | 128 KiB | 50 | 9715.34 | 5147.31 | 0.5298 |
| h2 | full | 128 KiB | 100 | 8809.32 | 4788.09 | 0.5435 |
| h2 | full | 128 KiB | 150 | 7860.68 | 4511.17 | 0.5739 |
| h2 | full | 1 MiB | 10 | 2898.11 | 910.68 | 0.3142 |
| h2 | full | 1 MiB | 50 | 2660.40 | 886.22 | 0.3331 |
| h2 | full | 1 MiB | 100 | 2481.55 | 821.98 | 0.3312 |
| h2 | full | 1 MiB | 150 | 2368.76 | 787.22 | 0.3323 |
| h2 | full | 2 MiB | 10 | 1548.32 | 444.53 | 0.2871 |
| h2 | full | 2 MiB | 50 | 1421.63 | 432.57 | 0.3043 |
| h2 | full | 2 MiB | 100 | 1299.56 | 409.47 | 0.3151 |
| h2 | full | 2 MiB | 150 | 1281.78 | 414.31 | 0.3232 |
| h2 | full | 4 MiB | 10 | 796.38 | 217.68 | 0.2733 |
| h2 | full | 4 MiB | 50 | 719.69 | 204.12 | 0.2836 |
| h2 | full | 4 MiB | 100 | 686.88 | 205.98 | 0.2999 |
| h2 | full | 4 MiB | 150 | 638.62 | 207.20 | 0.3244 |
| h2 | stream | 1 KiB | 10 | 8023.53 | 6914.84 | 0.8618 |
| h2 | stream | 1 KiB | 50 | 7176.06 | 6835.00 | 0.9525 |
| h2 | stream | 1 KiB | 100 | 6704.78 | 6957.40 | 1.0377 |
| h2 | stream | 1 KiB | 150 | 6272.29 | 6513.84 | 1.0385 |
| h2 | stream | 10 KiB | 10 | 8079.38 | 6346.94 | 0.7856 |
| h2 | stream | 10 KiB | 50 | 7241.27 | 6023.08 | 0.8318 |
| h2 | stream | 10 KiB | 100 | 6579.97 | 6018.20 | 0.9146 |
| h2 | stream | 10 KiB | 150 | 5998.76 | 6201.34 | 1.0338 |
| h2 | stream | 64 KiB | 10 | 3821.19 | 2968.29 | 0.7768 |
| h2 | stream | 64 KiB | 50 | 3672.85 | 2865.15 | 0.7801 |
| h2 | stream | 64 KiB | 100 | 3473.52 | 2709.97 | 0.7802 |
| h2 | stream | 64 KiB | 150 | 3365.22 | 2634.23 | 0.7828 |
| h2 | stream | 128 KiB | 10 | 3646.20 | 2423.42 | 0.6646 |
| h2 | stream | 128 KiB | 50 | 3567.04 | 2332.97 | 0.6540 |
| h2 | stream | 128 KiB | 100 | 3392.74 | 2182.93 | 0.6434 |
| h2 | stream | 128 KiB | 150 | 3273.59 | 2150.58 | 0.6569 |
| h2 | stream | 1 MiB | 10 | 1066.56 | 529.02 | 0.4960 |
| h2 | stream | 1 MiB | 50 | 1048.21 | 523.13 | 0.4991 |
| h2 | stream | 1 MiB | 100 | 1007.23 | 505.63 | 0.5020 |
| h2 | stream | 1 MiB | 150 | 994.17 | 482.90 | 0.4857 |
| h2 | stream | 2 MiB | 10 | 815.87 | 338.00 | 0.4143 |
| h2 | stream | 2 MiB | 50 | 798.75 | 331.54 | 0.4151 |
| h2 | stream | 2 MiB | 100 | 766.36 | 321.35 | 0.4193 |
| h2 | stream | 2 MiB | 150 | 749.78 | 308.06 | 0.4109 |
| h2 | stream | 4 MiB | 10 | 546.02 | 194.34 | 0.3559 |
| h2 | stream | 4 MiB | 50 | 530.02 | 186.05 | 0.3510 |
| h2 | stream | 4 MiB | 100 | 506.97 | 188.33 | 0.3715 |
| h2 | stream | 4 MiB | 150 | 509.19 | 181.97 | 0.3574 |

## Every common cell

| Client | Protocol | Upload | Payload | Concurrency | Before RPS | New RPS | Change (%) | Before CV (%) | New CV (%) |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| curl_cffi (blocking) | h1 | full | 1 KiB | 10 | 562.10 | 575.19 | +2.33 | 0.39 | 0.67 |
| curl_cffi (blocking) | h1 | full | 1 KiB | 50 | 557.93 | 576.50 | +3.33 | 0.71 | 0.40 |
| curl_cffi (blocking) | h1 | full | 1 KiB | 100 | 557.88 | 575.03 | +3.07 | 0.64 | 0.64 |
| curl_cffi (blocking) | h1 | full | 1 KiB | 150 | 552.02 | 568.10 | +2.91 | 0.41 | 0.13 |
| curl_cffi (blocking) | h1 | full | 10 KiB | 10 | 560.93 | 574.47 | +2.41 | 0.96 | 0.49 |
| curl_cffi (blocking) | h1 | full | 10 KiB | 50 | 565.20 | 575.69 | +1.86 | 0.19 | 0.35 |
| curl_cffi (blocking) | h1 | full | 10 KiB | 100 | 556.63 | 571.90 | +2.74 | 0.45 | 0.52 |
| curl_cffi (blocking) | h1 | full | 10 KiB | 150 | 551.68 | 563.08 | +2.07 | 0.34 | 0.48 |
| curl_cffi (blocking) | h1 | full | 64 KiB | 10 | 513.93 | 523.91 | +1.94 | 0.29 | 0.75 |
| curl_cffi (blocking) | h1 | full | 64 KiB | 50 | 507.21 | 529.07 | +4.31 | 1.03 | 0.13 |
| curl_cffi (blocking) | h1 | full | 64 KiB | 100 | 515.02 | 526.58 | +2.25 | 0.26 | 0.40 |
| curl_cffi (blocking) | h1 | full | 64 KiB | 150 | 508.78 | 519.72 | +2.15 | 0.28 | 0.56 |
| curl_cffi (blocking) | h1 | full | 128 KiB | 10 | 476.76 | 488.03 | +2.36 | 0.09 | 0.69 |
| curl_cffi (blocking) | h1 | full | 128 KiB | 50 | 479.38 | 491.49 | +2.53 | 0.29 | 0.13 |
| curl_cffi (blocking) | h1 | full | 128 KiB | 100 | 474.94 | 488.70 | +2.90 | 0.14 | 0.31 |
| curl_cffi (blocking) | h1 | full | 128 KiB | 150 | 471.13 | 482.14 | +2.34 | 0.88 | 0.33 |
| curl_cffi (blocking) | h1 | full | 1 MiB | 10 | 232.89 | 240.45 | +3.25 | 0.27 | 0.25 |
| curl_cffi (blocking) | h1 | full | 1 MiB | 50 | 234.42 | 238.46 | +1.72 | 0.12 | 0.50 |
| curl_cffi (blocking) | h1 | full | 1 MiB | 100 | 233.21 | 241.50 | +3.55 | 0.86 | 0.36 |
| curl_cffi (blocking) | h1 | full | 1 MiB | 150 | 241.08 | 244.91 | +1.59 | 0.46 | 0.27 |
| curl_cffi (blocking) | h1 | full | 2 MiB | 10 | 147.93 | 152.10 | +2.82 | 0.30 | 0.16 |
| curl_cffi (blocking) | h1 | full | 2 MiB | 50 | 147.96 | 152.15 | +2.83 | 0.40 | 0.22 |
| curl_cffi (blocking) | h1 | full | 2 MiB | 100 | 147.04 | 150.93 | +2.65 | 0.53 | 0.36 |
| curl_cffi (blocking) | h1 | full | 2 MiB | 150 | 153.08 | 156.46 | +2.20 | 0.75 | 0.13 |
| curl_cffi (blocking) | h1 | full | 4 MiB | 10 | 85.68 | 87.95 | +2.65 | 0.25 | 0.54 |
| curl_cffi (blocking) | h1 | full | 4 MiB | 50 | 84.70 | 87.43 | +3.22 | 0.55 | 0.64 |
| curl_cffi (blocking) | h1 | full | 4 MiB | 100 | 84.22 | 86.55 | +2.76 | 0.16 | 0.64 |
| curl_cffi (blocking) | h1 | full | 4 MiB | 150 | 88.28 | 89.91 | +1.84 | 0.45 | 0.24 |
| curl_cffi (blocking) | h1 | stream | 1 KiB | 10 | 526.74 | 540.17 | +2.55 | 0.84 | 0.12 |
| curl_cffi (blocking) | h1 | stream | 1 KiB | 50 | 530.93 | 540.42 | +1.79 | 0.25 | 0.06 |
| curl_cffi (blocking) | h1 | stream | 1 KiB | 100 | 528.18 | 539.42 | +2.13 | 0.21 | 0.32 |
| curl_cffi (blocking) | h1 | stream | 1 KiB | 150 | 515.15 | 532.07 | +3.28 | 1.85 | 0.76 |
| curl_cffi (blocking) | h1 | stream | 10 KiB | 10 | 526.20 | 539.37 | +2.50 | 0.72 | 0.37 |
| curl_cffi (blocking) | h1 | stream | 10 KiB | 50 | 527.06 | 543.58 | +3.13 | 1.48 | 0.81 |
| curl_cffi (blocking) | h1 | stream | 10 KiB | 100 | 523.66 | 535.22 | +2.21 | 0.51 | 0.32 |
| curl_cffi (blocking) | h1 | stream | 10 KiB | 150 | 520.89 | 532.30 | +2.19 | 0.44 | 0.60 |
| curl_cffi (blocking) | h1 | stream | 64 KiB | 10 | 458.46 | 472.81 | +3.13 | 1.16 | 0.34 |
| curl_cffi (blocking) | h1 | stream | 64 KiB | 50 | 462.19 | 470.60 | +1.82 | 0.69 | 0.31 |
| curl_cffi (blocking) | h1 | stream | 64 KiB | 100 | 459.64 | 472.21 | +2.74 | 0.18 | 0.31 |
| curl_cffi (blocking) | h1 | stream | 64 KiB | 150 | 461.72 | 468.44 | +1.46 | 0.40 | 0.41 |
| curl_cffi (blocking) | h1 | stream | 128 KiB | 10 | 431.00 | 440.21 | +2.14 | 0.14 | 0.57 |
| curl_cffi (blocking) | h1 | stream | 128 KiB | 50 | 431.34 | 444.55 | +3.06 | 0.14 | 0.72 |
| curl_cffi (blocking) | h1 | stream | 128 KiB | 100 | 428.82 | 438.92 | +2.36 | 0.35 | 0.50 |
| curl_cffi (blocking) | h1 | stream | 128 KiB | 150 | 430.24 | 437.88 | +1.77 | 0.10 | 0.07 |
| curl_cffi (blocking) | h1 | stream | 1 MiB | 10 | 180.09 | 185.03 | +2.74 | 0.51 | 0.62 |
| curl_cffi (blocking) | h1 | stream | 1 MiB | 50 | 178.09 | 184.19 | +3.42 | 0.15 | 0.11 |
| curl_cffi (blocking) | h1 | stream | 1 MiB | 100 | 176.38 | 182.58 | +3.51 | 0.15 | 0.45 |
| curl_cffi (blocking) | h1 | stream | 1 MiB | 150 | 182.86 | 186.42 | +1.95 | 0.39 | 0.15 |
| curl_cffi (blocking) | h1 | stream | 2 MiB | 10 | 116.14 | 118.99 | +2.46 | 0.43 | 0.40 |
| curl_cffi (blocking) | h1 | stream | 2 MiB | 50 | 113.86 | 117.14 | +2.88 | 0.51 | 0.53 |
| curl_cffi (blocking) | h1 | stream | 2 MiB | 100 | 112.70 | 115.56 | +2.53 | 0.62 | 0.09 |
| curl_cffi (blocking) | h1 | stream | 2 MiB | 150 | 117.28 | 119.72 | +2.08 | 0.25 | 0.21 |
| curl_cffi (blocking) | h1 | stream | 4 MiB | 10 | 67.55 | 69.66 | +3.13 | 0.95 | 0.09 |
| curl_cffi (blocking) | h1 | stream | 4 MiB | 50 | 66.12 | 67.95 | +2.76 | 0.13 | 0.39 |
| curl_cffi (blocking) | h1 | stream | 4 MiB | 100 | 64.53 | 67.37 | +4.40 | 0.24 | 0.29 |
| curl_cffi (blocking) | h1 | stream | 4 MiB | 150 | 68.04 | 69.29 | +1.84 | 0.17 | 0.39 |
| curl_cffi (blocking) | h2 | full | 1 KiB | 10 | 560.75 | 566.63 | +1.05 | 0.65 | 1.41 |
| curl_cffi (blocking) | h2 | full | 1 KiB | 50 | 561.48 | 577.79 | +2.91 | 0.18 | 0.19 |
| curl_cffi (blocking) | h2 | full | 1 KiB | 100 | 557.16 | 573.45 | +2.92 | 0.35 | 0.70 |
| curl_cffi (blocking) | h2 | full | 1 KiB | 150 | 551.51 | 564.84 | +2.42 | 0.17 | 0.35 |
| curl_cffi (blocking) | h2 | full | 10 KiB | 10 | 557.16 | 574.65 | +3.14 | 1.10 | 0.62 |
| curl_cffi (blocking) | h2 | full | 10 KiB | 50 | 564.14 | 574.77 | +1.88 | 0.38 | 0.50 |
| curl_cffi (blocking) | h2 | full | 10 KiB | 100 | 557.51 | 570.58 | +2.34 | 0.85 | 0.50 |
| curl_cffi (blocking) | h2 | full | 10 KiB | 150 | 546.60 | 562.09 | +2.84 | 1.25 | 0.50 |
| curl_cffi (blocking) | h2 | full | 64 KiB | 10 | 490.32 | 501.01 | +2.18 | 0.37 | 0.17 |
| curl_cffi (blocking) | h2 | full | 64 KiB | 50 | 490.35 | 502.35 | +2.45 | 0.59 | 0.98 |
| curl_cffi (blocking) | h2 | full | 64 KiB | 100 | 485.97 | 499.75 | +2.84 | 0.46 | 1.12 |
| curl_cffi (blocking) | h2 | full | 64 KiB | 150 | 481.96 | 493.27 | +2.35 | 0.56 | 0.43 |
| curl_cffi (blocking) | h2 | full | 128 KiB | 10 | 425.37 | 433.19 | +1.84 | 0.09 | 0.61 |
| curl_cffi (blocking) | h2 | full | 128 KiB | 50 | 426.64 | 436.70 | +2.36 | 0.59 | 0.56 |
| curl_cffi (blocking) | h2 | full | 128 KiB | 100 | 423.91 | 435.05 | +2.63 | 0.40 | 0.16 |
| curl_cffi (blocking) | h2 | full | 128 KiB | 150 | 422.81 | 431.02 | +1.94 | 0.27 | 0.26 |
| curl_cffi (blocking) | h2 | full | 1 MiB | 10 | 152.88 | 156.03 | +2.06 | 0.61 | 0.14 |
| curl_cffi (blocking) | h2 | full | 1 MiB | 50 | 152.57 | 156.89 | +2.83 | 0.42 | 0.60 |
| curl_cffi (blocking) | h2 | full | 1 MiB | 100 | 151.27 | 155.71 | +2.93 | 0.63 | 0.47 |
| curl_cffi (blocking) | h2 | full | 1 MiB | 150 | 156.13 | 158.14 | +1.29 | 0.35 | 0.19 |
| curl_cffi (blocking) | h2 | full | 2 MiB | 10 | 87.64 | 89.82 | +2.49 | 0.87 | 0.23 |
| curl_cffi (blocking) | h2 | full | 2 MiB | 50 | 88.13 | 89.78 | +1.88 | 0.15 | 0.32 |
| curl_cffi (blocking) | h2 | full | 2 MiB | 100 | 87.70 | 89.45 | +2.00 | 0.17 | 0.59 |
| curl_cffi (blocking) | h2 | full | 2 MiB | 150 | 90.16 | 91.75 | +1.76 | 0.42 | 0.16 |
| curl_cffi (blocking) | h2 | full | 4 MiB | 10 | 47.44 | 48.68 | +2.61 | 0.68 | 0.13 |
| curl_cffi (blocking) | h2 | full | 4 MiB | 50 | 47.58 | 48.66 | +2.27 | 0.25 | 0.65 |
| curl_cffi (blocking) | h2 | full | 4 MiB | 100 | 47.64 | 48.65 | +2.10 | 0.29 | 0.14 |
| curl_cffi (blocking) | h2 | full | 4 MiB | 150 | 48.98 | 49.58 | +1.22 | 0.30 | 0.08 |
| curl_cffi (blocking) | h2 | stream | 1 KiB | 10 | 528.52 | 540.84 | +2.33 | 0.32 | 0.25 |
| curl_cffi (blocking) | h2 | stream | 1 KiB | 50 | 526.48 | 540.76 | +2.71 | 0.28 | 0.19 |
| curl_cffi (blocking) | h2 | stream | 1 KiB | 100 | 525.29 | 539.35 | +2.68 | 0.29 | 0.46 |
| curl_cffi (blocking) | h2 | stream | 1 KiB | 150 | 516.36 | 533.72 | +3.36 | 0.46 | 0.37 |
| curl_cffi (blocking) | h2 | stream | 10 KiB | 10 | 527.22 | 538.23 | +2.09 | 0.76 | 0.51 |
| curl_cffi (blocking) | h2 | stream | 10 KiB | 50 | 524.94 | 542.29 | +3.31 | 0.96 | 0.41 |
| curl_cffi (blocking) | h2 | stream | 10 KiB | 100 | 527.92 | 538.86 | +2.07 | 0.51 | 0.48 |
| curl_cffi (blocking) | h2 | stream | 10 KiB | 150 | 516.09 | 534.71 | +3.61 | 1.32 | 0.50 |
| curl_cffi (blocking) | h2 | stream | 64 KiB | 10 | 443.83 | 446.87 | +0.69 | 0.30 | 0.89 |
| curl_cffi (blocking) | h2 | stream | 64 KiB | 50 | 438.63 | 451.79 | +3.00 | 0.77 | 0.69 |
| curl_cffi (blocking) | h2 | stream | 64 KiB | 100 | 441.07 | 450.17 | +2.06 | 0.57 | 0.61 |
| curl_cffi (blocking) | h2 | stream | 64 KiB | 150 | 439.40 | 445.85 | +1.47 | 0.50 | 0.31 |
| curl_cffi (blocking) | h2 | stream | 128 KiB | 10 | 384.85 | 396.57 | +3.05 | 0.56 | 0.69 |
| curl_cffi (blocking) | h2 | stream | 128 KiB | 50 | 388.38 | 398.34 | +2.57 | 1.16 | 0.30 |
| curl_cffi (blocking) | h2 | stream | 128 KiB | 100 | 387.62 | 397.17 | +2.46 | 0.61 | 0.34 |
| curl_cffi (blocking) | h2 | stream | 128 KiB | 150 | 389.16 | 394.79 | +1.45 | 0.28 | 0.15 |
| curl_cffi (blocking) | h2 | stream | 1 MiB | 10 | 136.83 | 140.13 | +2.42 | 0.16 | 0.59 |
| curl_cffi (blocking) | h2 | stream | 1 MiB | 50 | 137.27 | 140.42 | +2.29 | 0.18 | 0.71 |
| curl_cffi (blocking) | h2 | stream | 1 MiB | 100 | 135.67 | 138.43 | +2.03 | 0.51 | 0.36 |
| curl_cffi (blocking) | h2 | stream | 1 MiB | 150 | 138.95 | 141.40 | +1.76 | 0.50 | 0.26 |
| curl_cffi (blocking) | h2 | stream | 2 MiB | 10 | 78.51 | 80.48 | +2.50 | 0.19 | 0.29 |
| curl_cffi (blocking) | h2 | stream | 2 MiB | 50 | 78.26 | 80.01 | +2.24 | 0.45 | 0.50 |
| curl_cffi (blocking) | h2 | stream | 2 MiB | 100 | 77.61 | 78.81 | +1.55 | 0.63 | 0.36 |
| curl_cffi (blocking) | h2 | stream | 2 MiB | 150 | 79.21 | 80.96 | +2.21 | 0.67 | 0.35 |
| curl_cffi (blocking) | h2 | stream | 4 MiB | 10 | 41.94 | 43.32 | +3.30 | 0.71 | 0.07 |
| curl_cffi (blocking) | h2 | stream | 4 MiB | 50 | 41.86 | 42.93 | +2.56 | 1.48 | 0.16 |
| curl_cffi (blocking) | h2 | stream | 4 MiB | 100 | 41.60 | 42.41 | +1.95 | 0.29 | 0.63 |
| curl_cffi (blocking) | h2 | stream | 4 MiB | 150 | 42.82 | 43.41 | +1.37 | 0.18 | 0.40 |
| httpx (blocking) | h1 | full | 1 KiB | 10 | 1806.70 | 1847.26 | +2.25 | 1.67 | 0.96 |
| httpx (blocking) | h1 | full | 1 KiB | 50 | 1754.55 | 1824.53 | +3.99 | 1.00 | 1.04 |
| httpx (blocking) | h1 | full | 1 KiB | 100 | 1728.21 | 1774.98 | +2.71 | 1.01 | 0.11 |
| httpx (blocking) | h1 | full | 1 KiB | 150 | 1680.99 | 1713.04 | +1.91 | 0.96 | 0.35 |
| httpx (blocking) | h1 | full | 10 KiB | 10 | 1779.48 | 1831.58 | +2.93 | 0.85 | 0.67 |
| httpx (blocking) | h1 | full | 10 KiB | 50 | 1732.73 | 1818.54 | +4.95 | 1.11 | 1.56 |
| httpx (blocking) | h1 | full | 10 KiB | 100 | 1685.06 | 1766.21 | +4.82 | 0.89 | 0.60 |
| httpx (blocking) | h1 | full | 10 KiB | 150 | 1640.24 | 1739.36 | +6.04 | 3.51 | 1.61 |
| httpx (blocking) | h1 | full | 64 KiB | 10 | 1158.24 | 1190.82 | +2.81 | 0.44 | 0.28 |
| httpx (blocking) | h1 | full | 64 KiB | 50 | 1133.02 | 1157.69 | +2.18 | 0.95 | 1.06 |
| httpx (blocking) | h1 | full | 64 KiB | 100 | 1090.24 | 1121.42 | +2.86 | 0.27 | 0.47 |
| httpx (blocking) | h1 | full | 64 KiB | 150 | 1056.77 | 1074.73 | +1.70 | 0.69 | 3.38 |
| httpx (blocking) | h1 | full | 128 KiB | 10 | 858.01 | 873.39 | +1.79 | 0.84 | 0.91 |
| httpx (blocking) | h1 | full | 128 KiB | 50 | 835.17 | 862.05 | +3.22 | 0.57 | 0.45 |
| httpx (blocking) | h1 | full | 128 KiB | 100 | 817.87 | 835.74 | +2.18 | 0.91 | 0.90 |
| httpx (blocking) | h1 | full | 128 KiB | 150 | 801.82 | 815.25 | +1.67 | 0.60 | 3.13 |
| httpx (blocking) | h1 | full | 1 MiB | 10 | 178.77 | 183.21 | +2.49 | 0.79 | 1.03 |
| httpx (blocking) | h1 | full | 1 MiB | 50 | 182.10 | 185.83 | +2.05 | 1.05 | 0.33 |
| httpx (blocking) | h1 | full | 1 MiB | 100 | 178.39 | 184.26 | +3.29 | 1.12 | 0.30 |
| httpx (blocking) | h1 | full | 1 MiB | 150 | 183.17 | 188.40 | +2.85 | 0.48 | 0.18 |
| httpx (blocking) | h1 | full | 2 MiB | 10 | 94.91 | 96.48 | +1.65 | 0.71 | 0.38 |
| httpx (blocking) | h1 | full | 2 MiB | 50 | 95.74 | 97.93 | +2.28 | 0.58 | 0.20 |
| httpx (blocking) | h1 | full | 2 MiB | 100 | 94.81 | 96.84 | +2.14 | 0.49 | 1.17 |
| httpx (blocking) | h1 | full | 2 MiB | 150 | 97.73 | 99.58 | +1.90 | 0.09 | 0.23 |
| httpx (blocking) | h1 | full | 4 MiB | 10 | 48.58 | 49.67 | +2.25 | 1.04 | 0.18 |
| httpx (blocking) | h1 | full | 4 MiB | 50 | 49.73 | 50.83 | +2.20 | 0.62 | 0.26 |
| httpx (blocking) | h1 | full | 4 MiB | 100 | 48.88 | 50.20 | +2.71 | 0.51 | 0.50 |
| httpx (blocking) | h1 | full | 4 MiB | 150 | 50.05 | 51.39 | +2.68 | 0.78 | 0.35 |
| httpx (blocking) | h1 | stream | 1 KiB | 10 | 1532.63 | 1572.64 | +2.61 | 0.73 | 1.14 |
| httpx (blocking) | h1 | stream | 1 KiB | 50 | 1484.31 | 1540.21 | +3.77 | 1.33 | 0.20 |
| httpx (blocking) | h1 | stream | 1 KiB | 100 | 1465.71 | 1485.82 | +1.37 | 0.82 | 0.09 |
| httpx (blocking) | h1 | stream | 1 KiB | 150 | 1412.72 | 1466.26 | +3.79 | 0.41 | 0.80 |
| httpx (blocking) | h1 | stream | 10 KiB | 10 | 1530.15 | 1571.85 | +2.73 | 0.62 | 0.59 |
| httpx (blocking) | h1 | stream | 10 KiB | 50 | 1457.34 | 1528.82 | +4.91 | 1.98 | 0.91 |
| httpx (blocking) | h1 | stream | 10 KiB | 100 | 1447.16 | 1483.97 | +2.54 | 0.51 | 2.48 |
| httpx (blocking) | h1 | stream | 10 KiB | 150 | 1404.45 | 1460.47 | +3.99 | 1.33 | 0.96 |
| httpx (blocking) | h1 | stream | 64 KiB | 10 | 777.58 | 794.05 | +2.12 | 0.55 | 1.04 |
| httpx (blocking) | h1 | stream | 64 KiB | 50 | 759.27 | 789.85 | +4.03 | 1.28 | 1.06 |
| httpx (blocking) | h1 | stream | 64 KiB | 100 | 729.17 | 760.34 | +4.28 | 3.19 | 1.42 |
| httpx (blocking) | h1 | stream | 64 KiB | 150 | 737.76 | 766.16 | +3.85 | 1.11 | 0.38 |
| httpx (blocking) | h1 | stream | 128 KiB | 10 | 630.22 | 644.22 | +2.22 | 0.28 | 1.01 |
| httpx (blocking) | h1 | stream | 128 KiB | 50 | 617.61 | 633.07 | +2.50 | 0.70 | 0.19 |
| httpx (blocking) | h1 | stream | 128 KiB | 100 | 607.06 | 626.60 | +3.22 | 0.60 | 0.38 |
| httpx (blocking) | h1 | stream | 128 KiB | 150 | 595.97 | 616.42 | +3.43 | 0.41 | 0.21 |
| httpx (blocking) | h1 | stream | 1 MiB | 10 | 138.84 | 143.13 | +3.09 | 0.28 | 0.12 |
| httpx (blocking) | h1 | stream | 1 MiB | 50 | 139.03 | 143.34 | +3.09 | 0.81 | 0.32 |
| httpx (blocking) | h1 | stream | 1 MiB | 100 | 137.48 | 141.16 | +2.68 | 0.40 | 0.80 |
| httpx (blocking) | h1 | stream | 1 MiB | 150 | 140.79 | 143.84 | +2.16 | 0.74 | 0.56 |
| httpx (blocking) | h1 | stream | 2 MiB | 10 | 82.17 | 83.82 | +2.02 | 0.59 | 0.55 |
| httpx (blocking) | h1 | stream | 2 MiB | 50 | 82.82 | 85.01 | +2.65 | 0.25 | 0.15 |
| httpx (blocking) | h1 | stream | 2 MiB | 100 | 81.23 | 84.03 | +3.45 | 0.80 | 0.11 |
| httpx (blocking) | h1 | stream | 2 MiB | 150 | 83.46 | 85.30 | +2.21 | 0.27 | 0.36 |
| httpx (blocking) | h1 | stream | 4 MiB | 10 | 44.91 | 46.10 | +2.66 | 0.88 | 0.14 |
| httpx (blocking) | h1 | stream | 4 MiB | 50 | 45.58 | 46.81 | +2.70 | 0.94 | 0.61 |
| httpx (blocking) | h1 | stream | 4 MiB | 100 | 44.87 | 46.00 | +2.51 | 0.81 | 0.96 |
| httpx (blocking) | h1 | stream | 4 MiB | 150 | 45.98 | 47.53 | +3.35 | 0.98 | 0.23 |
| httpx (blocking) | h2 | full | 1 KiB | 10 | 1539.21 | 1592.76 | +3.48 | 1.96 | 0.20 |
| httpx (blocking) | h2 | full | 1 KiB | 50 | 1506.73 | 1544.25 | +2.49 | 1.86 | 0.38 |
| httpx (blocking) | h2 | full | 1 KiB | 100 | 1451.72 | 1492.07 | +2.78 | 1.23 | 1.72 |
| httpx (blocking) | h2 | full | 1 KiB | 150 | 1427.39 | 1410.79 | -1.16 | 0.76 | 3.31 |
| httpx (blocking) | h2 | full | 10 KiB | 10 | 1532.31 | 1569.32 | +2.41 | 1.07 | 0.15 |
| httpx (blocking) | h2 | full | 10 KiB | 50 | 1418.44 | 1530.83 | +7.92 | 5.54 | 0.47 |
| httpx (blocking) | h2 | full | 10 KiB | 100 | 1388.05 | 1474.82 | +6.25 | 5.13 | 0.52 |
| httpx (blocking) | h2 | full | 10 KiB | 150 | 1350.40 | 1410.86 | +4.48 | 3.96 | 4.23 |
| httpx (blocking) | h2 | full | 64 KiB | 10 | 617.12 | 630.90 | +2.23 | 0.40 | 0.40 |
| httpx (blocking) | h2 | full | 64 KiB | 50 | 591.75 | 602.61 | +1.84 | 2.77 | 2.73 |
| httpx (blocking) | h2 | full | 64 KiB | 100 | 587.86 | 604.64 | +2.85 | 0.31 | 0.43 |
| httpx (blocking) | h2 | full | 64 KiB | 150 | 596.23 | 610.88 | +2.46 | 0.77 | 0.64 |
| httpx (blocking) | h2 | full | 128 KiB | 10 | 356.08 | 369.62 | +3.80 | 0.21 | 0.50 |
| httpx (blocking) | h2 | full | 128 KiB | 50 | 357.74 | 360.95 | +0.90 | 0.79 | 1.52 |
| httpx (blocking) | h2 | full | 128 KiB | 100 | 349.88 | 359.66 | +2.80 | 0.60 | 1.13 |
| httpx (blocking) | h2 | full | 128 KiB | 150 | 353.01 | 363.06 | +2.85 | 0.58 | 0.19 |
| httpx (blocking) | h2 | full | 1 MiB | 10 | 50.61 | 51.28 | +1.32 | 0.80 | 0.96 |
| httpx (blocking) | h2 | full | 1 MiB | 50 | 48.51 | 50.52 | +4.13 | 0.75 | 0.63 |
| httpx (blocking) | h2 | full | 1 MiB | 100 | 48.31 | 49.56 | +2.59 | 0.36 | 0.79 |
| httpx (blocking) | h2 | full | 1 MiB | 150 | 50.74 | 51.93 | +2.34 | 0.22 | 0.43 |
| httpx (blocking) | h2 | full | 2 MiB | 10 | 24.56 | 24.98 | +1.69 | 0.09 | 0.71 |
| httpx (blocking) | h2 | full | 2 MiB | 50 | 22.89 | 23.65 | +3.33 | 0.65 | 0.55 |
| httpx (blocking) | h2 | full | 2 MiB | 100 | 22.72 | 23.39 | +2.95 | 0.15 | 0.04 |
| httpx (blocking) | h2 | full | 2 MiB | 150 | 23.64 | 24.11 | +1.99 | 0.35 | 0.68 |
| httpx (blocking) | h2 | full | 4 MiB | 10 | 10.91 | 11.18 | +2.47 | 0.73 | 0.44 |
| httpx (blocking) | h2 | full | 4 MiB | 50 | 10.01 | 10.30 | +2.85 | 0.53 | 0.44 |
| httpx (blocking) | h2 | full | 4 MiB | 100 | 9.90 | 10.17 | +2.73 | 0.55 | 0.44 |
| httpx (blocking) | h2 | full | 4 MiB | 150 | 10.16 | 10.45 | +2.85 | 0.40 | 0.48 |
| httpx (blocking) | h2 | stream | 1 KiB | 10 | 1539.86 | 1600.89 | +3.96 | 0.54 | 0.17 |
| httpx (blocking) | h2 | stream | 1 KiB | 50 | 1489.82 | 1487.85 | -0.13 | 0.85 | 2.52 |
| httpx (blocking) | h2 | stream | 1 KiB | 100 | 1452.39 | 1493.66 | +2.84 | 0.77 | 1.12 |
| httpx (blocking) | h2 | stream | 1 KiB | 150 | 1417.14 | 1456.43 | +2.77 | 0.65 | 1.65 |
| httpx (blocking) | h2 | stream | 10 KiB | 10 | 1519.08 | 1559.77 | +2.68 | 0.25 | 0.13 |
| httpx (blocking) | h2 | stream | 10 KiB | 50 | 1469.71 | 1520.15 | +3.43 | 0.85 | 0.96 |
| httpx (blocking) | h2 | stream | 10 KiB | 100 | 1442.28 | 1477.52 | +2.44 | 0.32 | 1.04 |
| httpx (blocking) | h2 | stream | 10 KiB | 150 | 1399.49 | 1447.43 | +3.42 | 0.31 | 0.53 |
| httpx (blocking) | h2 | stream | 64 KiB | 10 | 612.65 | 628.47 | +2.58 | 0.40 | 0.68 |
| httpx (blocking) | h2 | stream | 64 KiB | 50 | 613.92 | 627.65 | +2.24 | 0.97 | 1.10 |
| httpx (blocking) | h2 | stream | 64 KiB | 100 | 591.73 | 606.68 | +2.53 | 0.52 | 1.09 |
| httpx (blocking) | h2 | stream | 64 KiB | 150 | 595.58 | 609.92 | +2.41 | 0.42 | 0.18 |
| httpx (blocking) | h2 | stream | 128 KiB | 10 | 360.32 | 367.27 | +1.93 | 0.48 | 0.45 |
| httpx (blocking) | h2 | stream | 128 KiB | 50 | 358.88 | 368.66 | +2.73 | 0.21 | 0.87 |
| httpx (blocking) | h2 | stream | 128 KiB | 100 | 350.59 | 363.11 | +3.57 | 0.88 | 0.43 |
| httpx (blocking) | h2 | stream | 128 KiB | 150 | 351.14 | 367.00 | +4.51 | 1.43 | 0.57 |
| httpx (blocking) | h2 | stream | 1 MiB | 10 | 53.09 | 54.26 | +2.21 | 0.39 | 0.57 |
| httpx (blocking) | h2 | stream | 1 MiB | 50 | 53.26 | 53.92 | +1.23 | 0.60 | 0.31 |
| httpx (blocking) | h2 | stream | 1 MiB | 100 | 52.04 | 53.47 | +2.75 | 0.71 | 0.36 |
| httpx (blocking) | h2 | stream | 1 MiB | 150 | 54.42 | 55.50 | +1.98 | 0.40 | 0.08 |
| httpx (blocking) | h2 | stream | 2 MiB | 10 | 26.44 | 27.06 | +2.33 | 0.66 | 0.30 |
| httpx (blocking) | h2 | stream | 2 MiB | 50 | 26.28 | 27.15 | +3.29 | 1.10 | 0.41 |
| httpx (blocking) | h2 | stream | 2 MiB | 100 | 26.07 | 26.98 | +3.47 | 1.12 | 0.18 |
| httpx (blocking) | h2 | stream | 2 MiB | 150 | 27.46 | 27.87 | +1.49 | 0.15 | 0.39 |
| httpx (blocking) | h2 | stream | 4 MiB | 10 | 13.23 | 13.49 | +2.00 | 0.41 | 0.19 |
| httpx (blocking) | h2 | stream | 4 MiB | 50 | 13.32 | 13.55 | +1.73 | 0.62 | 0.42 |
| httpx (blocking) | h2 | stream | 4 MiB | 100 | 12.93 | 13.36 | +3.34 | 1.01 | 0.13 |
| httpx (blocking) | h2 | stream | 4 MiB | 150 | 13.65 | 13.91 | +1.84 | 0.12 | 0.51 |
| niquests (blocking) | h1 | full | 1 KiB | 10 | 1560.74 | 1594.92 | +2.19 | 0.88 | 1.83 |
| niquests (blocking) | h1 | full | 1 KiB | 50 | 1474.83 | 1570.62 | +6.50 | 0.33 | 1.51 |
| niquests (blocking) | h1 | full | 1 KiB | 100 | 1439.01 | 1535.70 | +6.72 | 2.12 | 0.69 |
| niquests (blocking) | h1 | full | 1 KiB | 150 | 1403.79 | 1468.09 | +4.58 | 1.56 | 0.97 |
| niquests (blocking) | h1 | full | 10 KiB | 10 | 1519.45 | 1524.65 | +0.34 | 0.26 | 5.50 |
| niquests (blocking) | h1 | full | 10 KiB | 50 | 1418.21 | 1551.05 | +9.37 | 8.67 | 1.51 |
| niquests (blocking) | h1 | full | 10 KiB | 100 | 1434.46 | 1485.88 | +3.58 | 0.49 | 2.08 |
| niquests (blocking) | h1 | full | 10 KiB | 150 | 1409.61 | 1450.13 | +2.87 | 0.58 | 1.40 |
| niquests (blocking) | h1 | full | 64 KiB | 10 | 1010.44 | 1093.09 | +8.18 | 4.18 | 1.83 |
| niquests (blocking) | h1 | full | 64 KiB | 50 | 979.29 | 1062.60 | +8.51 | 4.08 | 2.09 |
| niquests (blocking) | h1 | full | 64 KiB | 100 | 994.70 | 1035.13 | +4.06 | 1.59 | 0.86 |
| niquests (blocking) | h1 | full | 64 KiB | 150 | 967.85 | 1015.48 | +4.92 | 2.71 | 1.26 |
| niquests (blocking) | h1 | full | 128 KiB | 10 | 802.38 | 822.06 | +2.45 | 0.81 | 4.73 |
| niquests (blocking) | h1 | full | 128 KiB | 50 | 782.85 | 808.63 | +3.29 | 0.85 | 0.83 |
| niquests (blocking) | h1 | full | 128 KiB | 100 | 764.65 | 789.59 | +3.26 | 0.85 | 1.23 |
| niquests (blocking) | h1 | full | 128 KiB | 150 | 754.56 | 766.58 | +1.59 | 0.43 | 2.12 |
| niquests (blocking) | h1 | full | 1 MiB | 10 | 190.00 | 195.52 | +2.91 | 0.42 | 0.53 |
| niquests (blocking) | h1 | full | 1 MiB | 50 | 186.17 | 192.93 | +3.63 | 0.62 | 0.58 |
| niquests (blocking) | h1 | full | 1 MiB | 100 | 187.54 | 193.66 | +3.26 | 0.44 | 0.90 |
| niquests (blocking) | h1 | full | 1 MiB | 150 | 186.30 | 192.34 | +3.24 | 1.10 | 0.32 |
| niquests (blocking) | h1 | full | 2 MiB | 10 | 102.24 | 105.05 | +2.74 | 1.09 | 0.66 |
| niquests (blocking) | h1 | full | 2 MiB | 50 | 100.19 | 103.76 | +3.57 | 0.92 | 0.76 |
| niquests (blocking) | h1 | full | 2 MiB | 100 | 101.35 | 104.97 | +3.57 | 0.18 | 0.23 |
| niquests (blocking) | h1 | full | 2 MiB | 150 | 100.65 | 103.73 | +3.06 | 1.61 | 1.32 |
| niquests (blocking) | h1 | full | 4 MiB | 10 | 53.16 | 53.99 | +1.57 | 0.09 | 0.52 |
| niquests (blocking) | h1 | full | 4 MiB | 50 | 52.60 | 54.60 | +3.80 | 0.61 | 0.21 |
| niquests (blocking) | h1 | full | 4 MiB | 100 | 53.14 | 55.12 | +3.73 | 0.96 | 0.28 |
| niquests (blocking) | h1 | full | 4 MiB | 150 | 52.61 | 54.65 | +3.89 | 0.20 | 0.16 |
| niquests (blocking) | h1 | stream | 1 KiB | 10 | 1387.59 | 1436.49 | +3.52 | 1.13 | 0.65 |
| niquests (blocking) | h1 | stream | 1 KiB | 50 | 1358.62 | 1364.88 | +0.46 | 1.15 | 5.35 |
| niquests (blocking) | h1 | stream | 1 KiB | 100 | 1288.96 | 1363.03 | +5.75 | 0.81 | 0.30 |
| niquests (blocking) | h1 | stream | 1 KiB | 150 | 1285.04 | 1339.81 | +4.26 | 0.75 | 0.60 |
| niquests (blocking) | h1 | stream | 10 KiB | 10 | 1408.12 | 1426.24 | +1.29 | 0.44 | 0.94 |
| niquests (blocking) | h1 | stream | 10 KiB | 50 | 1353.34 | 1393.06 | +2.93 | 1.41 | 1.53 |
| niquests (blocking) | h1 | stream | 10 KiB | 100 | 1281.85 | 1351.57 | +5.44 | 1.82 | 1.29 |
| niquests (blocking) | h1 | stream | 10 KiB | 150 | 1277.87 | 1322.93 | +3.53 | 1.07 | 1.23 |
| niquests (blocking) | h1 | stream | 64 KiB | 10 | 807.19 | 823.65 | +2.04 | 0.30 | 0.64 |
| niquests (blocking) | h1 | stream | 64 KiB | 50 | 771.13 | 809.39 | +4.96 | 1.25 | 0.15 |
| niquests (blocking) | h1 | stream | 64 KiB | 100 | 752.44 | 791.30 | +5.16 | 0.54 | 0.58 |
| niquests (blocking) | h1 | stream | 64 KiB | 150 | 752.64 | 784.85 | +4.28 | 0.21 | 0.46 |
| niquests (blocking) | h1 | stream | 128 KiB | 10 | 655.05 | 678.89 | +3.64 | 0.64 | 0.35 |
| niquests (blocking) | h1 | stream | 128 KiB | 50 | 622.60 | 657.82 | +5.66 | 2.96 | 0.48 |
| niquests (blocking) | h1 | stream | 128 KiB | 100 | 623.96 | 646.63 | +3.63 | 0.68 | 0.23 |
| niquests (blocking) | h1 | stream | 128 KiB | 150 | 615.78 | 627.86 | +1.96 | 1.10 | 2.17 |
| niquests (blocking) | h1 | stream | 1 MiB | 10 | 152.46 | 159.12 | +4.37 | 0.79 | 0.39 |
| niquests (blocking) | h1 | stream | 1 MiB | 50 | 152.72 | 157.98 | +3.44 | 1.15 | 0.21 |
| niquests (blocking) | h1 | stream | 1 MiB | 100 | 153.22 | 158.90 | +3.71 | 0.39 | 0.56 |
| niquests (blocking) | h1 | stream | 1 MiB | 150 | 152.14 | 159.61 | +4.91 | 1.09 | 0.18 |
| niquests (blocking) | h1 | stream | 2 MiB | 10 | 90.61 | 93.36 | +3.03 | 0.72 | 0.37 |
| niquests (blocking) | h1 | stream | 2 MiB | 50 | 89.66 | 92.75 | +3.45 | 0.72 | 0.41 |
| niquests (blocking) | h1 | stream | 2 MiB | 100 | 90.69 | 94.09 | +3.75 | 0.85 | 0.43 |
| niquests (blocking) | h1 | stream | 2 MiB | 150 | 89.46 | 92.66 | +3.58 | 0.45 | 0.85 |
| niquests (blocking) | h1 | stream | 4 MiB | 10 | 49.43 | 50.75 | +2.66 | 0.73 | 0.31 |
| niquests (blocking) | h1 | stream | 4 MiB | 50 | 49.17 | 50.68 | +3.08 | 0.62 | 0.54 |
| niquests (blocking) | h1 | stream | 4 MiB | 100 | 49.85 | 51.81 | +3.94 | 0.28 | 0.29 |
| niquests (blocking) | h1 | stream | 4 MiB | 150 | 49.37 | 51.20 | +3.71 | 0.21 | 0.65 |
| niquests (blocking) | h2 | full | 1 KiB | 10 | 1304.07 | 1342.93 | +2.98 | 0.72 | 0.87 |
| niquests (blocking) | h2 | full | 1 KiB | 50 | 1265.69 | 1311.51 | +3.62 | 1.17 | 1.02 |
| niquests (blocking) | h2 | full | 1 KiB | 100 | 1180.08 | 1227.88 | +4.05 | 8.15 | 6.03 |
| niquests (blocking) | h2 | full | 1 KiB | 150 | 1202.86 | 1176.32 | -2.21 | 0.65 | 8.29 |
| niquests (blocking) | h2 | full | 10 KiB | 10 | 1289.84 | 1305.66 | +1.23 | 0.18 | 1.91 |
| niquests (blocking) | h2 | full | 10 KiB | 50 | 1243.51 | 1311.10 | +5.44 | 0.74 | 0.54 |
| niquests (blocking) | h2 | full | 10 KiB | 100 | 1218.28 | 1194.62 | -1.94 | 1.32 | 8.63 |
| niquests (blocking) | h2 | full | 10 KiB | 150 | 1097.90 | 1239.50 | +12.90 | 7.23 | 0.40 |
| niquests (blocking) | h2 | full | 64 KiB | 10 | 673.16 | 694.84 | +3.22 | 1.53 | 0.35 |
| niquests (blocking) | h2 | full | 64 KiB | 50 | 661.98 | 671.81 | +1.48 | 0.24 | 2.83 |
| niquests (blocking) | h2 | full | 64 KiB | 100 | 630.81 | 660.18 | +4.66 | 2.47 | 1.06 |
| niquests (blocking) | h2 | full | 64 KiB | 150 | 647.43 | 671.48 | +3.71 | 1.45 | 0.27 |
| niquests (blocking) | h2 | full | 128 KiB | 10 | 435.29 | 445.78 | +2.41 | 0.59 | 0.63 |
| niquests (blocking) | h2 | full | 128 KiB | 50 | 424.49 | 439.14 | +3.45 | 0.08 | 0.09 |
| niquests (blocking) | h2 | full | 128 KiB | 100 | 410.33 | 430.78 | +4.98 | 1.04 | 0.26 |
| niquests (blocking) | h2 | full | 128 KiB | 150 | 416.32 | 428.36 | +2.89 | 0.93 | 0.98 |
| niquests (blocking) | h2 | full | 1 MiB | 10 | 70.85 | 72.28 | +2.03 | 0.56 | 0.32 |
| niquests (blocking) | h2 | full | 1 MiB | 50 | 70.53 | 72.52 | +2.82 | 0.41 | 0.44 |
| niquests (blocking) | h2 | full | 1 MiB | 100 | 70.49 | 72.70 | +3.13 | 0.61 | 0.66 |
| niquests (blocking) | h2 | full | 1 MiB | 150 | 72.24 | 74.02 | +2.46 | 0.29 | 0.58 |
| niquests (blocking) | h2 | full | 2 MiB | 10 | 36.32 | 36.92 | +1.66 | 0.34 | 0.25 |
| niquests (blocking) | h2 | full | 2 MiB | 50 | 36.10 | 36.94 | +2.34 | 0.91 | 0.29 |
| niquests (blocking) | h2 | full | 2 MiB | 100 | 36.35 | 37.19 | +2.30 | 0.30 | 0.49 |
| niquests (blocking) | h2 | full | 2 MiB | 150 | 36.73 | 37.81 | +2.93 | 1.04 | 0.42 |
| niquests (blocking) | h2 | full | 4 MiB | 10 | 18.36 | 18.78 | +2.30 | 0.34 | 0.70 |
| niquests (blocking) | h2 | full | 4 MiB | 50 | 18.35 | 18.82 | +2.57 | 0.21 | 0.45 |
| niquests (blocking) | h2 | full | 4 MiB | 100 | 18.28 | 18.81 | +2.90 | 0.20 | 0.52 |
| niquests (blocking) | h2 | full | 4 MiB | 150 | 18.88 | 19.19 | +1.62 | 0.08 | 0.41 |
| niquests (blocking) | h2 | stream | 1 KiB | 10 | 1311.11 | 1316.83 | +0.44 | 0.54 | 0.47 |
| niquests (blocking) | h2 | stream | 1 KiB | 50 | 1224.51 | 1310.69 | +7.04 | 1.07 | 0.74 |
| niquests (blocking) | h2 | stream | 1 KiB | 100 | 1211.50 | 1258.40 | +3.87 | 1.21 | 1.25 |
| niquests (blocking) | h2 | stream | 1 KiB | 150 | 1221.42 | 1230.21 | +0.72 | 0.43 | 0.72 |
| niquests (blocking) | h2 | stream | 10 KiB | 10 | 1289.48 | 1252.30 | -2.88 | 0.96 | 7.09 |
| niquests (blocking) | h2 | stream | 10 KiB | 50 | 1276.77 | 1307.18 | +2.38 | 0.11 | 0.93 |
| niquests (blocking) | h2 | stream | 10 KiB | 100 | 1241.37 | 1259.10 | +1.43 | 0.67 | 0.55 |
| niquests (blocking) | h2 | stream | 10 KiB | 150 | 1173.17 | 1232.10 | +5.02 | 0.26 | 0.57 |
| niquests (blocking) | h2 | stream | 64 KiB | 10 | 654.26 | 691.99 | +5.77 | 4.45 | 0.42 |
| niquests (blocking) | h2 | stream | 64 KiB | 50 | 660.21 | 684.41 | +3.67 | 1.99 | 0.49 |
| niquests (blocking) | h2 | stream | 64 KiB | 100 | 649.39 | 668.82 | +2.99 | 0.30 | 0.57 |
| niquests (blocking) | h2 | stream | 64 KiB | 150 | 642.71 | 665.40 | +3.53 | 0.51 | 0.57 |
| niquests (blocking) | h2 | stream | 128 KiB | 10 | 432.56 | 439.91 | +1.70 | 0.52 | 0.24 |
| niquests (blocking) | h2 | stream | 128 KiB | 50 | 419.58 | 432.30 | +3.03 | 0.65 | 0.33 |
| niquests (blocking) | h2 | stream | 128 KiB | 100 | 414.16 | 423.08 | +2.15 | 0.20 | 0.97 |
| niquests (blocking) | h2 | stream | 128 KiB | 150 | 410.70 | 424.69 | +3.41 | 3.38 | 0.49 |
| niquests (blocking) | h2 | stream | 1 MiB | 10 | 70.28 | 71.49 | +1.73 | 1.06 | 0.54 |
| niquests (blocking) | h2 | stream | 1 MiB | 50 | 70.15 | 71.99 | +2.63 | 0.59 | 0.24 |
| niquests (blocking) | h2 | stream | 1 MiB | 100 | 69.78 | 72.10 | +3.32 | 1.23 | 0.13 |
| niquests (blocking) | h2 | stream | 1 MiB | 150 | 72.24 | 73.37 | +1.57 | 0.11 | 0.55 |
| niquests (blocking) | h2 | stream | 2 MiB | 10 | 36.07 | 36.80 | +2.02 | 0.80 | 0.71 |
| niquests (blocking) | h2 | stream | 2 MiB | 50 | 36.10 | 36.90 | +2.21 | 0.23 | 0.26 |
| niquests (blocking) | h2 | stream | 2 MiB | 100 | 36.17 | 36.89 | +2.01 | 0.46 | 0.48 |
| niquests (blocking) | h2 | stream | 2 MiB | 150 | 36.94 | 37.79 | +2.31 | 0.79 | 0.67 |
| niquests (blocking) | h2 | stream | 4 MiB | 10 | 18.31 | 18.72 | +2.25 | 0.17 | 0.60 |
| niquests (blocking) | h2 | stream | 4 MiB | 50 | 18.24 | 18.75 | +2.80 | 0.03 | 0.31 |
| niquests (blocking) | h2 | stream | 4 MiB | 100 | 18.22 | 18.75 | +2.87 | 0.46 | 0.33 |
| niquests (blocking) | h2 | stream | 4 MiB | 150 | 18.72 | 19.20 | +2.58 | 0.58 | 0.49 |
| PycURL | h1 | full | 1 KiB | 10 | 13052.16 | 13665.05 | +4.70 | 3.03 | 1.45 |
| PycURL | h1 | full | 1 KiB | 50 | 11415.68 | 12144.96 | +6.39 | 1.18 | 0.57 |
| PycURL | h1 | full | 1 KiB | 100 | 10297.65 | 10765.99 | +4.55 | 2.07 | 0.50 |
| PycURL | h1 | full | 1 KiB | 150 | 9214.98 | 9753.95 | +5.85 | 4.34 | 1.12 |
| PycURL | h1 | full | 10 KiB | 10 | 13170.18 | 13486.06 | +2.40 | 0.64 | 0.34 |
| PycURL | h1 | full | 10 KiB | 50 | 11388.53 | 12091.50 | +6.17 | 1.63 | 1.12 |
| PycURL | h1 | full | 10 KiB | 100 | 10496.03 | 10544.77 | +0.46 | 1.09 | 1.09 |
| PycURL | h1 | full | 10 KiB | 150 | 9392.16 | 9567.57 | +1.87 | 0.33 | 0.34 |
| PycURL | h1 | full | 64 KiB | 10 | 4639.04 | 4752.35 | +2.44 | 0.89 | 1.44 |
| PycURL | h1 | full | 64 KiB | 50 | 4214.47 | 4468.45 | +6.03 | 4.56 | 1.17 |
| PycURL | h1 | full | 64 KiB | 100 | 4132.08 | 4221.41 | +2.16 | 0.61 | 1.00 |
| PycURL | h1 | full | 64 KiB | 150 | 3850.23 | 4071.42 | +5.74 | 1.62 | 0.76 |
| PycURL | h1 | full | 128 KiB | 10 | 2728.89 | 2841.59 | +4.13 | 1.07 | 0.78 |
| PycURL | h1 | full | 128 KiB | 50 | 2697.27 | 2770.35 | +2.71 | 0.20 | 0.58 |
| PycURL | h1 | full | 128 KiB | 100 | 2563.26 | 2627.80 | +2.52 | 0.67 | 0.36 |
| PycURL | h1 | full | 128 KiB | 150 | 2471.45 | 2500.47 | +1.17 | 1.60 | 2.50 |
| PycURL | h1 | full | 1 MiB | 10 | 421.03 | 433.08 | +2.86 | 0.41 | 1.14 |
| PycURL | h1 | full | 1 MiB | 50 | 414.75 | 440.52 | +6.21 | 1.61 | 1.42 |
| PycURL | h1 | full | 1 MiB | 100 | 415.10 | 426.28 | +2.69 | 0.85 | 0.57 |
| PycURL | h1 | full | 1 MiB | 150 | 414.61 | 426.11 | +2.77 | 0.33 | 0.23 |
| PycURL | h1 | full | 2 MiB | 10 | 213.88 | 220.94 | +3.30 | 0.76 | 0.46 |
| PycURL | h1 | full | 2 MiB | 50 | 222.79 | 226.77 | +1.79 | 0.29 | 0.80 |
| PycURL | h1 | full | 2 MiB | 100 | 220.12 | 224.62 | +2.05 | 2.60 | 1.44 |
| PycURL | h1 | full | 2 MiB | 150 | 212.73 | 218.23 | +2.59 | 1.26 | 0.42 |
| PycURL | h1 | full | 4 MiB | 10 | 108.75 | 110.93 | +2.01 | 0.86 | 0.29 |
| PycURL | h1 | full | 4 MiB | 50 | 112.98 | 114.01 | +0.91 | 1.62 | 0.42 |
| PycURL | h1 | full | 4 MiB | 100 | 115.05 | 114.42 | -0.55 | 1.05 | 0.99 |
| PycURL | h1 | full | 4 MiB | 150 | 108.89 | 113.91 | +4.61 | 1.60 | 3.82 |
| PycURL | h1 | stream | 1 KiB | 10 | 6789.64 | 6981.08 | +2.82 | 1.27 | 1.92 |
| PycURL | h1 | stream | 1 KiB | 50 | 6446.44 | 6733.18 | +4.45 | 0.52 | 0.81 |
| PycURL | h1 | stream | 1 KiB | 100 | 5984.62 | 6207.81 | +3.73 | 2.17 | 1.71 |
| PycURL | h1 | stream | 1 KiB | 150 | 5712.43 | 5838.07 | +2.20 | 0.30 | 2.50 |
| PycURL | h1 | stream | 10 KiB | 10 | 6820.35 | 7062.69 | +3.55 | 1.34 | 2.07 |
| PycURL | h1 | stream | 10 KiB | 50 | 6330.45 | 6614.01 | +4.48 | 0.45 | 0.83 |
| PycURL | h1 | stream | 10 KiB | 100 | 6068.56 | 6283.23 | +3.54 | 0.68 | 0.31 |
| PycURL | h1 | stream | 10 KiB | 150 | 5703.72 | 5907.77 | +3.58 | 1.34 | 1.81 |
| PycURL | h1 | stream | 64 KiB | 10 | 2480.34 | 2561.77 | +3.28 | 0.30 | 0.75 |
| PycURL | h1 | stream | 64 KiB | 50 | 2479.25 | 2615.39 | +5.49 | 0.93 | 0.79 |
| PycURL | h1 | stream | 64 KiB | 100 | 2409.30 | 2456.80 | +1.97 | 0.60 | 0.66 |
| PycURL | h1 | stream | 64 KiB | 150 | 2320.84 | 2380.51 | +2.57 | 1.19 | 0.40 |
| PycURL | h1 | stream | 128 KiB | 10 | 1828.44 | 1908.66 | +4.39 | 1.39 | 0.71 |
| PycURL | h1 | stream | 128 KiB | 50 | 1820.36 | 1847.33 | +1.48 | 0.30 | 1.29 |
| PycURL | h1 | stream | 128 KiB | 100 | 1763.66 | 1811.83 | +2.73 | 0.78 | 0.66 |
| PycURL | h1 | stream | 128 KiB | 150 | 1723.19 | 1750.60 | +1.59 | 1.42 | 0.22 |
| PycURL | h1 | stream | 1 MiB | 10 | 277.87 | 287.88 | +3.60 | 0.21 | 0.29 |
| PycURL | h1 | stream | 1 MiB | 50 | 283.33 | 291.77 | +2.98 | 0.39 | 0.54 |
| PycURL | h1 | stream | 1 MiB | 100 | 278.27 | 287.46 | +3.30 | 0.65 | 0.51 |
| PycURL | h1 | stream | 1 MiB | 150 | 279.62 | 286.09 | +2.31 | 0.27 | 0.17 |
| PycURL | h1 | stream | 2 MiB | 10 | 153.51 | 158.27 | +3.10 | 0.61 | 0.40 |
| PycURL | h1 | stream | 2 MiB | 50 | 156.48 | 161.15 | +2.99 | 0.63 | 0.36 |
| PycURL | h1 | stream | 2 MiB | 100 | 151.60 | 160.01 | +5.55 | 2.27 | 0.23 |
| PycURL | h1 | stream | 2 MiB | 150 | 156.38 | 159.70 | +2.12 | 0.51 | 0.12 |
| PycURL | h1 | stream | 4 MiB | 10 | 80.86 | 83.81 | +3.65 | 0.20 | 0.71 |
| PycURL | h1 | stream | 4 MiB | 50 | 82.77 | 85.11 | +2.83 | 0.23 | 0.28 |
| PycURL | h1 | stream | 4 MiB | 100 | 81.47 | 84.36 | +3.55 | 0.45 | 0.35 |
| PycURL | h1 | stream | 4 MiB | 150 | 82.84 | 84.52 | +2.03 | 0.16 | 0.40 |
| PycURL | h2 | full | 1 KiB | 10 | 12681.07 | 13463.29 | +6.17 | 2.13 | 1.86 |
| PycURL | h2 | full | 1 KiB | 50 | 11447.19 | 11955.20 | +4.44 | 1.07 | 0.27 |
| PycURL | h2 | full | 1 KiB | 100 | 10066.68 | 10600.33 | +5.30 | 1.23 | 1.02 |
| PycURL | h2 | full | 1 KiB | 150 | 9220.00 | 9411.65 | +2.08 | 1.62 | 1.68 |
| PycURL | h2 | full | 10 KiB | 10 | 12739.63 | 12941.25 | +1.58 | 1.82 | 0.79 |
| PycURL | h2 | full | 10 KiB | 50 | 11210.24 | 11752.31 | +4.84 | 2.67 | 0.62 |
| PycURL | h2 | full | 10 KiB | 100 | 10268.38 | 10281.93 | +0.13 | 1.31 | 1.12 |
| PycURL | h2 | full | 10 KiB | 150 | 9174.41 | 9483.98 | +3.37 | 1.45 | 0.37 |
| PycURL | h2 | full | 64 KiB | 10 | 3001.30 | 3118.75 | +3.91 | 0.71 | 0.77 |
| PycURL | h2 | full | 64 KiB | 50 | 3045.74 | 3145.56 | +3.28 | 1.00 | 0.32 |
| PycURL | h2 | full | 64 KiB | 100 | 2805.22 | 2924.25 | +4.24 | 3.90 | 1.78 |
| PycURL | h2 | full | 64 KiB | 150 | 2770.10 | 2884.23 | +4.12 | 3.14 | 0.83 |
| PycURL | h2 | full | 128 KiB | 10 | 1614.36 | 1648.83 | +2.14 | 1.22 | 0.49 |
| PycURL | h2 | full | 128 KiB | 50 | 1636.27 | 1687.94 | +3.16 | 0.64 | 0.92 |
| PycURL | h2 | full | 128 KiB | 100 | 1605.94 | 1650.77 | +2.79 | 0.45 | 0.66 |
| PycURL | h2 | full | 128 KiB | 150 | 1563.41 | 1596.18 | +2.10 | 0.67 | 1.31 |
| PycURL | h2 | full | 1 MiB | 10 | 212.54 | 220.44 | +3.71 | 0.31 | 0.78 |
| PycURL | h2 | full | 1 MiB | 50 | 220.25 | 227.53 | +3.30 | 1.44 | 0.85 |
| PycURL | h2 | full | 1 MiB | 100 | 218.77 | 225.70 | +3.17 | 0.91 | 0.48 |
| PycURL | h2 | full | 1 MiB | 150 | 219.59 | 226.23 | +3.03 | 0.32 | 0.21 |
| PycURL | h2 | full | 2 MiB | 10 | 106.97 | 109.84 | +2.68 | 0.56 | 0.24 |
| PycURL | h2 | full | 2 MiB | 50 | 110.75 | 113.94 | +2.87 | 0.77 | 0.54 |
| PycURL | h2 | full | 2 MiB | 100 | 112.50 | 114.33 | +1.63 | 0.61 | 1.00 |
| PycURL | h2 | full | 2 MiB | 150 | 111.13 | 116.58 | +4.90 | 0.39 | 1.70 |
| PycURL | h2 | full | 4 MiB | 10 | 53.35 | 54.83 | +2.79 | 0.15 | 0.74 |
| PycURL | h2 | full | 4 MiB | 50 | 55.56 | 57.38 | +3.28 | 1.05 | 0.53 |
| PycURL | h2 | full | 4 MiB | 100 | 56.15 | 58.30 | +3.83 | 0.80 | 1.21 |
| PycURL | h2 | full | 4 MiB | 150 | 57.76 | 58.62 | +1.48 | 1.37 | 0.16 |
| PycURL | h2 | stream | 1 KiB | 10 | 6757.96 | 6923.86 | +2.45 | 1.84 | 2.12 |
| PycURL | h2 | stream | 1 KiB | 50 | 6265.21 | 6590.77 | +5.20 | 1.37 | 0.64 |
| PycURL | h2 | stream | 1 KiB | 100 | 6017.54 | 6218.92 | +3.35 | 0.70 | 0.12 |
| PycURL | h2 | stream | 1 KiB | 150 | 5702.83 | 5699.80 | -0.05 | 1.39 | 4.36 |
| PycURL | h2 | stream | 10 KiB | 10 | 6817.39 | 6854.61 | +0.55 | 2.56 | 1.15 |
| PycURL | h2 | stream | 10 KiB | 50 | 6289.54 | 6374.16 | +1.35 | 1.52 | 0.30 |
| PycURL | h2 | stream | 10 KiB | 100 | 5975.39 | 6187.62 | +3.55 | 2.04 | 0.83 |
| PycURL | h2 | stream | 10 KiB | 150 | 5510.61 | 5755.71 | +4.45 | 2.19 | 0.77 |
| PycURL | h2 | stream | 64 KiB | 10 | 1957.48 | 2044.70 | +4.46 | 0.85 | 0.22 |
| PycURL | h2 | stream | 64 KiB | 50 | 1984.92 | 2023.23 | +1.93 | 0.88 | 0.21 |
| PycURL | h2 | stream | 64 KiB | 100 | 1892.19 | 1953.37 | +3.23 | 1.43 | 1.46 |
| PycURL | h2 | stream | 64 KiB | 150 | 1827.37 | 1927.96 | +5.50 | 3.17 | 0.06 |
| PycURL | h2 | stream | 128 KiB | 10 | 1234.62 | 1285.36 | +4.11 | 1.75 | 0.75 |
| PycURL | h2 | stream | 128 KiB | 50 | 1260.40 | 1306.01 | +3.62 | 1.36 | 0.46 |
| PycURL | h2 | stream | 128 KiB | 100 | 1229.43 | 1272.82 | +3.53 | 1.40 | 1.20 |
| PycURL | h2 | stream | 128 KiB | 150 | 1221.39 | 1246.86 | +2.09 | 0.92 | 0.47 |
| PycURL | h2 | stream | 1 MiB | 10 | 186.62 | 192.08 | +2.92 | 0.89 | 0.84 |
| PycURL | h2 | stream | 1 MiB | 50 | 191.83 | 198.40 | +3.42 | 0.71 | 0.25 |
| PycURL | h2 | stream | 1 MiB | 100 | 191.30 | 196.40 | +2.67 | 0.47 | 0.33 |
| PycURL | h2 | stream | 1 MiB | 150 | 193.22 | 197.15 | +2.03 | 0.30 | 0.39 |
| PycURL | h2 | stream | 2 MiB | 10 | 93.67 | 96.97 | +3.52 | 0.27 | 0.18 |
| PycURL | h2 | stream | 2 MiB | 50 | 97.75 | 100.59 | +2.90 | 0.32 | 0.45 |
| PycURL | h2 | stream | 2 MiB | 100 | 97.11 | 99.76 | +2.74 | 0.47 | 0.18 |
| PycURL | h2 | stream | 2 MiB | 150 | 97.74 | 99.91 | +2.22 | 0.17 | 0.28 |
| PycURL | h2 | stream | 4 MiB | 10 | 47.50 | 48.91 | +2.95 | 0.69 | 0.42 |
| PycURL | h2 | stream | 4 MiB | 50 | 49.03 | 50.35 | +2.69 | 0.31 | 0.42 |
| PycURL | h2 | stream | 4 MiB | 100 | 48.39 | 50.03 | +3.39 | 0.22 | 0.48 |
| PycURL | h2 | stream | 4 MiB | 150 | 49.20 | 50.29 | +2.22 | 0.49 | 0.34 |
| requests | h1 | full | 1 KiB | 10 | 1434.25 | 1513.25 | +5.51 | 2.74 | 0.67 |
| requests | h1 | full | 1 KiB | 50 | 1413.44 | 1460.29 | +3.31 | 0.84 | 1.14 |
| requests | h1 | full | 1 KiB | 100 | 1375.73 | 1409.92 | +2.49 | 0.28 | 1.04 |
| requests | h1 | full | 1 KiB | 150 | 1318.62 | 1383.91 | +4.95 | 2.67 | 1.21 |
| requests | h1 | full | 10 KiB | 10 | 1458.34 | 1509.34 | +3.50 | 1.20 | 1.26 |
| requests | h1 | full | 10 KiB | 50 | 1385.99 | 1442.40 | +4.07 | 1.61 | 2.47 |
| requests | h1 | full | 10 KiB | 100 | 1348.55 | 1427.00 | +5.82 | 1.14 | 1.23 |
| requests | h1 | full | 10 KiB | 150 | 1339.74 | 1404.12 | +4.81 | 0.41 | 0.94 |
| requests | h1 | full | 64 KiB | 10 | 994.68 | 1034.02 | +3.95 | 0.97 | 0.71 |
| requests | h1 | full | 64 KiB | 50 | 978.56 | 1020.20 | +4.25 | 1.26 | 1.06 |
| requests | h1 | full | 64 KiB | 100 | 927.65 | 989.28 | +6.64 | 4.12 | 0.49 |
| requests | h1 | full | 64 KiB | 150 | 943.39 | 986.10 | +4.53 | 1.35 | 0.98 |
| requests | h1 | full | 128 KiB | 10 | 789.92 | 804.83 | +1.89 | 1.08 | 0.29 |
| requests | h1 | full | 128 KiB | 50 | 742.66 | 792.44 | +6.70 | 3.48 | 0.49 |
| requests | h1 | full | 128 KiB | 100 | 740.52 | 778.68 | +5.15 | 1.64 | 0.08 |
| requests | h1 | full | 128 KiB | 150 | 738.12 | 763.25 | +3.41 | 0.78 | 0.53 |
| requests | h1 | full | 1 MiB | 10 | 191.03 | 194.40 | +1.76 | 1.72 | 0.77 |
| requests | h1 | full | 1 MiB | 50 | 184.75 | 193.29 | +4.62 | 2.16 | 0.52 |
| requests | h1 | full | 1 MiB | 100 | 187.62 | 192.82 | +2.77 | 0.52 | 0.78 |
| requests | h1 | full | 1 MiB | 150 | 185.27 | 192.24 | +3.76 | 0.92 | 0.59 |
| requests | h1 | full | 2 MiB | 10 | 102.22 | 104.57 | +2.30 | 0.23 | 0.71 |
| requests | h1 | full | 2 MiB | 50 | 102.12 | 104.01 | +1.85 | 0.29 | 0.47 |
| requests | h1 | full | 2 MiB | 100 | 101.95 | 105.50 | +3.48 | 0.23 | 0.50 |
| requests | h1 | full | 2 MiB | 150 | 101.45 | 104.41 | +2.91 | 0.71 | 0.07 |
| requests | h1 | full | 4 MiB | 10 | 53.28 | 54.16 | +1.66 | 0.51 | 0.66 |
| requests | h1 | full | 4 MiB | 50 | 53.01 | 54.48 | +2.78 | 0.84 | 0.61 |
| requests | h1 | full | 4 MiB | 100 | 53.96 | 55.38 | +2.64 | 0.45 | 0.52 |
| requests | h1 | full | 4 MiB | 150 | 53.13 | 54.97 | +3.46 | 0.05 | 0.46 |
| requests | h1 | stream | 1 KiB | 10 | 1313.72 | 1370.10 | +4.29 | 1.08 | 0.98 |
| requests | h1 | stream | 1 KiB | 50 | 1284.52 | 1334.04 | +3.85 | 0.57 | 1.90 |
| requests | h1 | stream | 1 KiB | 100 | 1219.48 | 1240.80 | +1.75 | 1.30 | 3.68 |
| requests | h1 | stream | 1 KiB | 150 | 1195.39 | 1269.40 | +6.19 | 0.99 | 1.30 |
| requests | h1 | stream | 10 KiB | 10 | 1316.03 | 1370.17 | +4.11 | 0.66 | 0.60 |
| requests | h1 | stream | 10 KiB | 50 | 1260.04 | 1317.88 | +4.59 | 0.43 | 0.35 |
| requests | h1 | stream | 10 KiB | 100 | 1236.67 | 1287.74 | +4.13 | 0.71 | 0.24 |
| requests | h1 | stream | 10 KiB | 150 | 1204.01 | 1254.37 | +4.18 | 1.60 | 1.26 |
| requests | h1 | stream | 64 KiB | 10 | 780.88 | 801.59 | +2.65 | 0.90 | 1.02 |
| requests | h1 | stream | 64 KiB | 50 | 758.24 | 776.44 | +2.40 | 1.15 | 3.10 |
| requests | h1 | stream | 64 KiB | 100 | 735.27 | 762.87 | +3.75 | 0.46 | 0.76 |
| requests | h1 | stream | 64 KiB | 150 | 715.57 | 760.80 | +6.32 | 2.77 | 0.16 |
| requests | h1 | stream | 128 KiB | 10 | 640.53 | 655.75 | +2.38 | 0.70 | 0.63 |
| requests | h1 | stream | 128 KiB | 50 | 620.66 | 639.28 | +3.00 | 0.77 | 0.69 |
| requests | h1 | stream | 128 KiB | 100 | 607.14 | 629.90 | +3.75 | 0.34 | 0.73 |
| requests | h1 | stream | 128 KiB | 150 | 586.27 | 625.01 | +6.61 | 2.58 | 0.46 |
| requests | h1 | stream | 1 MiB | 10 | 155.07 | 159.27 | +2.71 | 0.41 | 0.32 |
| requests | h1 | stream | 1 MiB | 50 | 153.84 | 157.66 | +2.49 | 0.54 | 0.16 |
| requests | h1 | stream | 1 MiB | 100 | 154.31 | 158.15 | +2.49 | 0.16 | 0.68 |
| requests | h1 | stream | 1 MiB | 150 | 152.54 | 158.69 | +4.03 | 0.52 | 0.54 |
| requests | h1 | stream | 2 MiB | 10 | 91.42 | 92.44 | +1.11 | 0.30 | 0.10 |
| requests | h1 | stream | 2 MiB | 50 | 90.31 | 93.20 | +3.19 | 0.86 | 0.54 |
| requests | h1 | stream | 2 MiB | 100 | 91.41 | 94.07 | +2.90 | 0.49 | 0.19 |
| requests | h1 | stream | 2 MiB | 150 | 91.13 | 93.92 | +3.06 | 0.48 | 0.73 |
| requests | h1 | stream | 4 MiB | 10 | 49.80 | 50.60 | +1.60 | 0.37 | 0.83 |
| requests | h1 | stream | 4 MiB | 50 | 49.26 | 50.98 | +3.50 | 0.88 | 0.13 |
| requests | h1 | stream | 4 MiB | 100 | 50.56 | 52.19 | +3.22 | 0.22 | 0.27 |
| requests | h1 | stream | 4 MiB | 150 | 49.98 | 51.53 | +3.10 | 0.58 | 0.47 |
| ry (blocking) | h1 | full | 1 KiB | 10 | 9057.47 | 8792.55 | -2.92 | 0.75 | 6.53 |
| ry (blocking) | h1 | full | 1 KiB | 50 | 7899.31 | 8322.61 | +5.36 | 2.97 | 0.60 |
| ry (blocking) | h1 | full | 1 KiB | 100 | 7385.52 | 7645.47 | +3.52 | 0.67 | 2.10 |
| ry (blocking) | h1 | full | 1 KiB | 150 | 6837.76 | 7100.59 | +3.84 | 0.76 | 0.10 |
| ry (blocking) | h1 | full | 10 KiB | 10 | 8685.16 | 9171.58 | +5.60 | 1.51 | 1.57 |
| ry (blocking) | h1 | full | 10 KiB | 50 | 7971.43 | 8318.86 | +4.36 | 1.50 | 0.57 |
| ry (blocking) | h1 | full | 10 KiB | 100 | 7454.23 | 7531.72 | +1.04 | 1.33 | 1.64 |
| ry (blocking) | h1 | full | 10 KiB | 150 | 6420.64 | 6863.37 | +6.90 | 7.32 | 1.29 |
| ry (blocking) | h1 | full | 64 KiB | 10 | 8911.36 | 9103.77 | +2.16 | 0.92 | 2.03 |
| ry (blocking) | h1 | full | 64 KiB | 50 | 7805.62 | 8085.39 | +3.58 | 0.20 | 2.17 |
| ry (blocking) | h1 | full | 64 KiB | 100 | 7190.68 | 7516.17 | +4.53 | 1.87 | 1.03 |
| ry (blocking) | h1 | full | 64 KiB | 150 | 6513.28 | 6930.57 | +6.41 | 2.72 | 1.39 |
| ry (blocking) | h1 | full | 128 KiB | 10 | 8538.42 | 8876.46 | +3.96 | 2.79 | 0.40 |
| ry (blocking) | h1 | full | 128 KiB | 50 | 7569.82 | 7801.26 | +3.06 | 0.27 | 1.56 |
| ry (blocking) | h1 | full | 128 KiB | 100 | 6969.66 | 7203.26 | +3.35 | 1.58 | 1.06 |
| ry (blocking) | h1 | full | 128 KiB | 150 | 6205.77 | 6635.89 | +6.93 | 1.24 | 0.44 |
| ry (blocking) | h1 | full | 1 MiB | 10 | 4779.48 | 4847.17 | +1.42 | 2.75 | 4.79 |
| ry (blocking) | h1 | full | 1 MiB | 50 | 4017.17 | 4292.92 | +6.86 | 1.12 | 2.90 |
| ry (blocking) | h1 | full | 1 MiB | 100 | 3584.99 | 3709.31 | +3.47 | 1.75 | 0.43 |
| ry (blocking) | h1 | full | 1 MiB | 150 | 3349.30 | 3476.38 | +3.79 | 2.36 | 1.10 |
| ry (blocking) | h1 | full | 2 MiB | 10 | 2079.02 | 2212.81 | +6.44 | 3.68 | 4.91 |
| ry (blocking) | h1 | full | 2 MiB | 50 | 1953.06 | 2043.50 | +4.63 | 1.24 | 1.50 |
| ry (blocking) | h1 | full | 2 MiB | 100 | 1812.45 | 1934.58 | +6.74 | 2.13 | 2.17 |
| ry (blocking) | h1 | full | 2 MiB | 150 | 1744.51 | 1896.72 | +8.73 | 4.10 | 3.12 |
| ry (blocking) | h1 | full | 4 MiB | 10 | 912.61 | 1065.45 | +16.75 | 7.91 | 1.38 |
| ry (blocking) | h1 | full | 4 MiB | 50 | 931.70 | 992.53 | +6.53 | 2.54 | 0.77 |
| ry (blocking) | h1 | full | 4 MiB | 100 | 864.25 | 951.91 | +10.14 | 1.67 | 1.99 |
| ry (blocking) | h1 | full | 4 MiB | 150 | 801.20 | 876.24 | +9.37 | 0.67 | 1.38 |
| ry (blocking) | h1 | stream | 1 KiB | 10 | 5381.01 | 5574.99 | +3.60 | 0.26 | 1.73 |
| ry (blocking) | h1 | stream | 1 KiB | 50 | 5072.09 | 5234.88 | +3.21 | 1.58 | 1.16 |
| ry (blocking) | h1 | stream | 1 KiB | 100 | 4781.39 | 5028.80 | +5.17 | 0.29 | 0.29 |
| ry (blocking) | h1 | stream | 1 KiB | 150 | 4566.79 | 4693.88 | +2.78 | 0.68 | 1.95 |
| ry (blocking) | h1 | stream | 10 KiB | 10 | 5347.39 | 5650.37 | +5.67 | 1.43 | 0.82 |
| ry (blocking) | h1 | stream | 10 KiB | 50 | 5051.68 | 5283.22 | +4.58 | 0.16 | 0.43 |
| ry (blocking) | h1 | stream | 10 KiB | 100 | 4773.64 | 4960.80 | +3.92 | 0.77 | 1.96 |
| ry (blocking) | h1 | stream | 10 KiB | 150 | 4548.47 | 4750.90 | +4.45 | 2.18 | 0.80 |
| ry (blocking) | h1 | stream | 64 KiB | 10 | 3445.37 | 3511.56 | +1.92 | 1.06 | 0.87 |
| ry (blocking) | h1 | stream | 64 KiB | 50 | 3306.79 | 3374.86 | +2.06 | 0.56 | 1.38 |
| ry (blocking) | h1 | stream | 64 KiB | 100 | 3150.07 | 3228.20 | +2.48 | 1.81 | 1.41 |
| ry (blocking) | h1 | stream | 64 KiB | 150 | 3027.09 | 3119.51 | +3.05 | 0.35 | 1.57 |
| ry (blocking) | h1 | stream | 128 KiB | 10 | 3431.12 | 3480.54 | +1.44 | 1.25 | 0.67 |
| ry (blocking) | h1 | stream | 128 KiB | 50 | 3305.71 | 3389.08 | +2.52 | 0.21 | 1.02 |
| ry (blocking) | h1 | stream | 128 KiB | 100 | 3109.34 | 3203.89 | +3.04 | 0.73 | 1.53 |
| ry (blocking) | h1 | stream | 128 KiB | 150 | 2972.43 | 3038.11 | +2.21 | 1.15 | 0.47 |
| ry (blocking) | h1 | stream | 1 MiB | 10 | 1198.88 | 1251.30 | +4.37 | 0.26 | 0.52 |
| ry (blocking) | h1 | stream | 1 MiB | 50 | 1215.73 | 1254.33 | +3.17 | 0.91 | 0.69 |
| ry (blocking) | h1 | stream | 1 MiB | 100 | 1167.48 | 1213.78 | +3.97 | 0.40 | 0.91 |
| ry (blocking) | h1 | stream | 1 MiB | 150 | 1129.17 | 1162.91 | +2.99 | 0.98 | 0.13 |
| ry (blocking) | h1 | stream | 2 MiB | 10 | 1101.44 | 1088.79 | -1.15 | 5.82 | 2.86 |
| ry (blocking) | h1 | stream | 2 MiB | 50 | 999.84 | 1031.77 | +3.19 | 0.60 | 1.29 |
| ry (blocking) | h1 | stream | 2 MiB | 100 | 964.24 | 1005.27 | +4.26 | 0.63 | 0.37 |
| ry (blocking) | h1 | stream | 2 MiB | 150 | 942.96 | 980.58 | +3.99 | 0.39 | 0.66 |
| ry (blocking) | h1 | stream | 4 MiB | 10 | 882.44 | 956.29 | +8.37 | 10.08 | 9.53 |
| ry (blocking) | h1 | stream | 4 MiB | 50 | 756.16 | 769.31 | +1.74 | 0.61 | 0.59 |
| ry (blocking) | h1 | stream | 4 MiB | 100 | 741.04 | 750.09 | +1.22 | 0.42 | 0.57 |
| ry (blocking) | h1 | stream | 4 MiB | 150 | 719.22 | 739.45 | +2.81 | 0.90 | 0.56 |
| ry (blocking) | h2 | full | 1 KiB | 10 | 8857.65 | 9378.86 | +5.88 | 1.98 | 1.00 |
| ry (blocking) | h2 | full | 1 KiB | 50 | 7950.76 | 8174.78 | +2.82 | 1.37 | 0.54 |
| ry (blocking) | h2 | full | 1 KiB | 100 | 7365.06 | 7612.04 | +3.35 | 1.03 | 1.50 |
| ry (blocking) | h2 | full | 1 KiB | 150 | 6708.04 | 6996.15 | +4.29 | 1.48 | 1.27 |
| ry (blocking) | h2 | full | 10 KiB | 10 | 8745.05 | 8936.85 | +2.19 | 1.03 | 1.16 |
| ry (blocking) | h2 | full | 10 KiB | 50 | 8049.29 | 8046.92 | -0.03 | 0.31 | 0.68 |
| ry (blocking) | h2 | full | 10 KiB | 100 | 7225.68 | 7635.18 | +5.67 | 0.92 | 1.13 |
| ry (blocking) | h2 | full | 10 KiB | 150 | 6718.16 | 6964.18 | +3.66 | 2.73 | 0.52 |
| ry (blocking) | h2 | full | 64 KiB | 10 | 4607.33 | 4753.29 | +3.17 | 2.32 | 0.98 |
| ry (blocking) | h2 | full | 64 KiB | 50 | 4322.58 | 4455.07 | +3.07 | 0.74 | 0.85 |
| ry (blocking) | h2 | full | 64 KiB | 100 | 4119.76 | 4133.56 | +0.33 | 1.02 | 1.23 |
| ry (blocking) | h2 | full | 64 KiB | 150 | 3830.57 | 3998.99 | +4.40 | 2.42 | 1.51 |
| ry (blocking) | h2 | full | 128 KiB | 10 | 2821.66 | 2857.02 | +1.25 | 1.59 | 1.65 |
| ry (blocking) | h2 | full | 128 KiB | 50 | 2656.70 | 2729.52 | +2.74 | 0.51 | 0.56 |
| ry (blocking) | h2 | full | 128 KiB | 100 | 2512.74 | 2604.06 | +3.63 | 0.97 | 1.06 |
| ry (blocking) | h2 | full | 128 KiB | 150 | 2477.05 | 2534.51 | +2.32 | 1.12 | 0.79 |
| ry (blocking) | h2 | full | 1 MiB | 10 | 418.67 | 440.30 | +5.17 | 0.43 | 1.66 |
| ry (blocking) | h2 | full | 1 MiB | 50 | 422.28 | 436.49 | +3.36 | 0.54 | 0.53 |
| ry (blocking) | h2 | full | 1 MiB | 100 | 416.26 | 431.34 | +3.62 | 0.49 | 0.34 |
| ry (blocking) | h2 | full | 1 MiB | 150 | 416.08 | 426.50 | +2.50 | 1.06 | 0.39 |
| ry (blocking) | h2 | full | 2 MiB | 10 | 213.05 | 217.99 | +2.32 | 0.44 | 0.55 |
| ry (blocking) | h2 | full | 2 MiB | 50 | 222.41 | 225.75 | +1.50 | 1.36 | 0.85 |
| ry (blocking) | h2 | full | 2 MiB | 100 | 222.83 | 227.91 | +2.28 | 2.05 | 1.46 |
| ry (blocking) | h2 | full | 2 MiB | 150 | 217.79 | 228.69 | +5.01 | 1.13 | 3.14 |
| ry (blocking) | h2 | full | 4 MiB | 10 | 104.72 | 108.81 | +3.91 | 0.17 | 0.17 |
| ry (blocking) | h2 | full | 4 MiB | 50 | 108.10 | 111.02 | +2.69 | 0.86 | 0.72 |
| ry (blocking) | h2 | full | 4 MiB | 100 | 109.43 | 111.65 | +2.03 | 0.89 | 1.26 |
| ry (blocking) | h2 | full | 4 MiB | 150 | 110.29 | 112.58 | +2.08 | 0.85 | 0.65 |
| ry (blocking) | h2 | stream | 1 KiB | 10 | 5391.81 | 5565.85 | +3.23 | 0.59 | 0.85 |
| ry (blocking) | h2 | stream | 1 KiB | 50 | 5065.00 | 5221.57 | +3.09 | 1.19 | 1.43 |
| ry (blocking) | h2 | stream | 1 KiB | 100 | 4773.48 | 4997.51 | +4.69 | 0.89 | 0.85 |
| ry (blocking) | h2 | stream | 1 KiB | 150 | 4523.32 | 4678.60 | +3.43 | 0.60 | 0.87 |
| ry (blocking) | h2 | stream | 10 KiB | 10 | 5345.37 | 5497.50 | +2.85 | 1.52 | 2.37 |
| ry (blocking) | h2 | stream | 10 KiB | 50 | 5031.97 | 5219.12 | +3.72 | 0.63 | 0.18 |
| ry (blocking) | h2 | stream | 10 KiB | 100 | 4676.09 | 4947.79 | +5.81 | 1.04 | 0.97 |
| ry (blocking) | h2 | stream | 10 KiB | 150 | 4519.74 | 4647.67 | +2.83 | 1.30 | 1.66 |
| ry (blocking) | h2 | stream | 64 KiB | 10 | 2491.07 | 2570.42 | +3.19 | 2.49 | 0.79 |
| ry (blocking) | h2 | stream | 64 KiB | 50 | 2467.75 | 2529.48 | +2.50 | 0.58 | 0.50 |
| ry (blocking) | h2 | stream | 64 KiB | 100 | 2374.13 | 2425.33 | +2.16 | 0.20 | 0.49 |
| ry (blocking) | h2 | stream | 64 KiB | 150 | 2297.35 | 2395.91 | +4.29 | 1.15 | 0.25 |
| ry (blocking) | h2 | stream | 128 KiB | 10 | 1847.46 | 1901.04 | +2.90 | 1.06 | 0.93 |
| ry (blocking) | h2 | stream | 128 KiB | 50 | 1821.48 | 1885.17 | +3.50 | 0.89 | 0.45 |
| ry (blocking) | h2 | stream | 128 KiB | 100 | 1747.08 | 1797.09 | +2.86 | 0.20 | 0.09 |
| ry (blocking) | h2 | stream | 128 KiB | 150 | 1717.92 | 1759.14 | +2.40 | 1.21 | 0.53 |
| ry (blocking) | h2 | stream | 1 MiB | 10 | 334.65 | 344.21 | +2.86 | 0.49 | 0.75 |
| ry (blocking) | h2 | stream | 1 MiB | 50 | 338.52 | 345.24 | +1.98 | 0.88 | 0.62 |
| ry (blocking) | h2 | stream | 1 MiB | 100 | 335.77 | 343.57 | +2.32 | 0.09 | 0.14 |
| ry (blocking) | h2 | stream | 1 MiB | 150 | 329.61 | 340.89 | +3.42 | 1.00 | 0.54 |
| ry (blocking) | h2 | stream | 2 MiB | 10 | 187.98 | 192.99 | +2.67 | 0.47 | 0.12 |
| ry (blocking) | h2 | stream | 2 MiB | 50 | 190.76 | 196.91 | +3.22 | 0.34 | 0.39 |
| ry (blocking) | h2 | stream | 2 MiB | 100 | 190.86 | 195.19 | +2.27 | 0.33 | 0.13 |
| ry (blocking) | h2 | stream | 2 MiB | 150 | 189.96 | 195.15 | +2.73 | 0.14 | 0.16 |
| ry (blocking) | h2 | stream | 4 MiB | 10 | 98.14 | 101.95 | +3.88 | 0.49 | 0.23 |
| ry (blocking) | h2 | stream | 4 MiB | 50 | 100.50 | 103.58 | +3.07 | 0.32 | 0.60 |
| ry (blocking) | h2 | stream | 4 MiB | 100 | 99.36 | 102.85 | +3.51 | 0.69 | 0.50 |
| ry (blocking) | h2 | stream | 4 MiB | 150 | 101.78 | 103.92 | +2.10 | 0.09 | 0.31 |
| wreq (blocking MT) | h1 | full | 1 KiB | 10 | 18996.45 | 19830.18 | +4.39 | 0.88 | 2.16 |
| wreq (blocking MT) | h1 | full | 1 KiB | 50 | 15439.60 | 15894.76 | +2.95 | 1.60 | 1.84 |
| wreq (blocking MT) | h1 | full | 1 KiB | 100 | 12985.92 | 13270.06 | +2.19 | 1.02 | 1.65 |
| wreq (blocking MT) | h1 | full | 1 KiB | 150 | 10964.74 | 11482.57 | +4.72 | 1.66 | 1.76 |
| wreq (blocking MT) | h1 | full | 10 KiB | 10 | 19292.58 | 19323.55 | +0.16 | 2.26 | 3.43 |
| wreq (blocking MT) | h1 | full | 10 KiB | 50 | 14865.63 | 15960.32 | +7.36 | 4.08 | 1.63 |
| wreq (blocking MT) | h1 | full | 10 KiB | 100 | 12714.75 | 13033.26 | +2.50 | 2.67 | 1.75 |
| wreq (blocking MT) | h1 | full | 10 KiB | 150 | 11128.46 | 11442.41 | +2.82 | 2.67 | 1.03 |
| wreq (blocking MT) | h1 | full | 64 KiB | 10 | 17525.24 | 18691.14 | +6.65 | 0.88 | 2.23 |
| wreq (blocking MT) | h1 | full | 64 KiB | 50 | 14336.07 | 14933.17 | +4.17 | 0.48 | 2.03 |
| wreq (blocking MT) | h1 | full | 64 KiB | 100 | 11964.96 | 12640.69 | +5.65 | 1.69 | 1.03 |
| wreq (blocking MT) | h1 | full | 64 KiB | 150 | 10692.50 | 10995.99 | +2.84 | 0.59 | 1.64 |
| wreq (blocking MT) | h1 | full | 128 KiB | 10 | 16787.47 | 17637.09 | +5.06 | 1.17 | 1.50 |
| wreq (blocking MT) | h1 | full | 128 KiB | 50 | 13752.95 | 14479.16 | +5.28 | 1.70 | 1.38 |
| wreq (blocking MT) | h1 | full | 128 KiB | 100 | 11767.68 | 12134.56 | +3.12 | 0.86 | 1.11 |
| wreq (blocking MT) | h1 | full | 128 KiB | 150 | 10078.55 | 10090.43 | +0.12 | 1.32 | 1.04 |
| wreq (blocking MT) | h1 | full | 1 MiB | 10 | 4836.67 | 4987.94 | +3.13 | 3.76 | 10.37 |
| wreq (blocking MT) | h1 | full | 1 MiB | 50 | 4158.81 | 4359.47 | +4.82 | 1.16 | 1.43 |
| wreq (blocking MT) | h1 | full | 1 MiB | 100 | 3863.36 | 4040.45 | +4.58 | 0.84 | 1.56 |
| wreq (blocking MT) | h1 | full | 1 MiB | 150 | 3689.88 | 3895.27 | +5.57 | 0.39 | 3.62 |
| wreq (blocking MT) | h1 | full | 2 MiB | 10 | 1977.09 | 2240.33 | +13.31 | 3.72 | 6.28 |
| wreq (blocking MT) | h1 | full | 2 MiB | 50 | 1990.07 | 2069.77 | +4.00 | 0.58 | 1.20 |
| wreq (blocking MT) | h1 | full | 2 MiB | 100 | 1866.96 | 1938.86 | +3.85 | 0.58 | 2.21 |
| wreq (blocking MT) | h1 | full | 2 MiB | 150 | 1745.24 | 1852.60 | +6.15 | 0.09 | 1.54 |
| wreq (blocking MT) | h1 | full | 4 MiB | 10 | 986.91 | 1023.24 | +3.68 | 2.89 | 3.66 |
| wreq (blocking MT) | h1 | full | 4 MiB | 50 | 957.89 | 995.50 | +3.93 | 0.51 | 1.28 |
| wreq (blocking MT) | h1 | full | 4 MiB | 100 | 901.98 | 954.77 | +5.85 | 1.61 | 1.80 |
| wreq (blocking MT) | h1 | full | 4 MiB | 150 | 794.06 | 883.57 | +11.27 | 3.39 | 1.33 |
| wreq (blocking MT) | h1 | stream | 1 KiB | 10 | 8133.88 | 8128.62 | -0.06 | 1.06 | 1.75 |
| wreq (blocking MT) | h1 | stream | 1 KiB | 50 | 7405.33 | 7156.76 | -3.36 | 0.94 | 1.06 |
| wreq (blocking MT) | h1 | stream | 1 KiB | 100 | 6918.36 | 6681.92 | -3.42 | 0.60 | 0.31 |
| wreq (blocking MT) | h1 | stream | 1 KiB | 150 | 6368.86 | 6287.02 | -1.28 | 0.97 | 0.24 |
| wreq (blocking MT) | h1 | stream | 10 KiB | 10 | 8044.44 | 8178.71 | +1.67 | 1.54 | 1.29 |
| wreq (blocking MT) | h1 | stream | 10 KiB | 50 | 7203.05 | 7190.70 | -0.17 | 1.89 | 1.41 |
| wreq (blocking MT) | h1 | stream | 10 KiB | 100 | 6931.83 | 6733.02 | -2.87 | 0.46 | 1.38 |
| wreq (blocking MT) | h1 | stream | 10 KiB | 150 | 6257.83 | 6192.03 | -1.05 | 2.86 | 0.84 |
| wreq (blocking MT) | h1 | stream | 64 KiB | 10 | 4191.08 | 4396.31 | +4.90 | 1.52 | 0.32 |
| wreq (blocking MT) | h1 | stream | 64 KiB | 50 | 4097.11 | 4120.12 | +0.56 | 0.67 | 1.34 |
| wreq (blocking MT) | h1 | stream | 64 KiB | 100 | 3901.64 | 3883.44 | -0.47 | 0.89 | 0.59 |
| wreq (blocking MT) | h1 | stream | 64 KiB | 150 | 3668.06 | 3697.82 | +0.81 | 2.43 | 2.10 |
| wreq (blocking MT) | h1 | stream | 128 KiB | 10 | 4133.62 | 4343.64 | +5.08 | 1.70 | 1.49 |
| wreq (blocking MT) | h1 | stream | 128 KiB | 50 | 4023.39 | 4038.40 | +0.37 | 0.62 | 0.64 |
| wreq (blocking MT) | h1 | stream | 128 KiB | 100 | 3839.07 | 3854.86 | +0.41 | 0.49 | 1.40 |
| wreq (blocking MT) | h1 | stream | 128 KiB | 150 | 3571.23 | 3637.20 | +1.85 | 0.34 | 1.93 |
| wreq (blocking MT) | h1 | stream | 1 MiB | 10 | 1321.24 | 1427.21 | +8.02 | 0.68 | 0.82 |
| wreq (blocking MT) | h1 | stream | 1 MiB | 50 | 1320.68 | 1386.86 | +5.01 | 1.12 | 0.69 |
| wreq (blocking MT) | h1 | stream | 1 MiB | 100 | 1253.33 | 1334.69 | +6.49 | 0.99 | 0.26 |
| wreq (blocking MT) | h1 | stream | 1 MiB | 150 | 1250.06 | 1319.64 | +5.57 | 0.86 | 0.56 |
| wreq (blocking MT) | h1 | stream | 2 MiB | 10 | 1196.98 | 1279.74 | +6.91 | 1.02 | 0.55 |
| wreq (blocking MT) | h1 | stream | 2 MiB | 50 | 1208.62 | 1265.62 | +4.72 | 0.44 | 0.40 |
| wreq (blocking MT) | h1 | stream | 2 MiB | 100 | 1147.58 | 1199.23 | +4.50 | 0.72 | 0.77 |
| wreq (blocking MT) | h1 | stream | 2 MiB | 150 | 1129.65 | 1177.09 | +4.20 | 1.01 | 0.74 |
| wreq (blocking MT) | h1 | stream | 4 MiB | 10 | 936.47 | 947.88 | +1.22 | 6.03 | 5.83 |
| wreq (blocking MT) | h1 | stream | 4 MiB | 50 | 894.08 | 951.77 | +6.45 | 1.54 | 0.70 |
| wreq (blocking MT) | h1 | stream | 4 MiB | 100 | 813.13 | 881.18 | +8.37 | 1.49 | 0.16 |
| wreq (blocking MT) | h1 | stream | 4 MiB | 150 | 808.78 | 872.29 | +7.85 | 1.68 | 0.98 |
| wreq (blocking MT) | h2 | full | 1 KiB | 10 | 17755.26 | 19206.81 | +8.18 | 3.28 | 0.42 |
| wreq (blocking MT) | h2 | full | 1 KiB | 50 | 14903.83 | 15566.42 | +4.45 | 0.67 | 2.06 |
| wreq (blocking MT) | h2 | full | 1 KiB | 100 | 12628.79 | 13215.84 | +4.65 | 0.57 | 1.24 |
| wreq (blocking MT) | h2 | full | 1 KiB | 150 | 11127.18 | 11335.87 | +1.88 | 1.49 | 4.21 |
| wreq (blocking MT) | h2 | full | 10 KiB | 10 | 18764.47 | 19127.76 | +1.94 | 0.97 | 0.05 |
| wreq (blocking MT) | h2 | full | 10 KiB | 50 | 14929.32 | 15439.76 | +3.42 | 0.49 | 1.91 |
| wreq (blocking MT) | h2 | full | 10 KiB | 100 | 12632.74 | 13039.35 | +3.22 | 0.58 | 1.32 |
| wreq (blocking MT) | h2 | full | 10 KiB | 150 | 10987.77 | 11341.20 | +3.22 | 1.88 | 1.78 |
| wreq (blocking MT) | h2 | full | 64 KiB | 10 | 11308.26 | 11867.41 | +4.94 | 1.90 | 0.36 |
| wreq (blocking MT) | h2 | full | 64 KiB | 50 | 9783.89 | 10195.32 | +4.21 | 2.09 | 1.73 |
| wreq (blocking MT) | h2 | full | 64 KiB | 100 | 9002.50 | 9162.33 | +1.78 | 0.34 | 2.35 |
| wreq (blocking MT) | h2 | full | 64 KiB | 150 | 8101.04 | 8306.01 | +2.53 | 2.59 | 1.84 |
| wreq (blocking MT) | h2 | full | 128 KiB | 10 | 10217.14 | 10963.26 | +7.30 | 2.12 | 1.71 |
| wreq (blocking MT) | h2 | full | 128 KiB | 50 | 9443.86 | 9715.34 | +2.87 | 1.94 | 0.32 |
| wreq (blocking MT) | h2 | full | 128 KiB | 100 | 8615.55 | 8809.32 | +2.25 | 0.37 | 2.23 |
| wreq (blocking MT) | h2 | full | 128 KiB | 150 | 7540.76 | 7860.68 | +4.24 | 2.34 | 0.81 |
| wreq (blocking MT) | h2 | full | 1 MiB | 10 | 2831.64 | 2898.11 | +2.35 | 0.82 | 0.31 |
| wreq (blocking MT) | h2 | full | 1 MiB | 50 | 2563.49 | 2660.40 | +3.78 | 3.93 | 1.42 |
| wreq (blocking MT) | h2 | full | 1 MiB | 100 | 2470.19 | 2481.55 | +0.46 | 2.02 | 0.09 |
| wreq (blocking MT) | h2 | full | 1 MiB | 150 | 2282.00 | 2368.76 | +3.80 | 1.23 | 2.28 |
| wreq (blocking MT) | h2 | full | 2 MiB | 10 | 1521.77 | 1548.32 | +1.74 | 1.24 | 0.98 |
| wreq (blocking MT) | h2 | full | 2 MiB | 50 | 1403.70 | 1421.63 | +1.28 | 0.70 | 0.58 |
| wreq (blocking MT) | h2 | full | 2 MiB | 100 | 1269.99 | 1299.56 | +2.33 | 0.73 | 1.70 |
| wreq (blocking MT) | h2 | full | 2 MiB | 150 | 1244.65 | 1281.78 | +2.98 | 0.61 | 0.20 |
| wreq (blocking MT) | h2 | full | 4 MiB | 10 | 768.12 | 796.38 | +3.68 | 0.83 | 1.79 |
| wreq (blocking MT) | h2 | full | 4 MiB | 50 | 716.10 | 719.69 | +0.50 | 1.20 | 1.90 |
| wreq (blocking MT) | h2 | full | 4 MiB | 100 | 662.92 | 686.88 | +3.61 | 2.09 | 0.45 |
| wreq (blocking MT) | h2 | full | 4 MiB | 150 | 622.45 | 638.62 | +2.60 | 3.13 | 2.87 |
| wreq (blocking MT) | h2 | stream | 1 KiB | 10 | 7786.75 | 8023.53 | +3.04 | 5.75 | 1.04 |
| wreq (blocking MT) | h2 | stream | 1 KiB | 50 | 7403.74 | 7176.06 | -3.08 | 2.09 | 1.00 |
| wreq (blocking MT) | h2 | stream | 1 KiB | 100 | 6651.98 | 6704.78 | +0.79 | 3.88 | 1.05 |
| wreq (blocking MT) | h2 | stream | 1 KiB | 150 | 6281.03 | 6272.29 | -0.14 | 0.65 | 0.89 |
| wreq (blocking MT) | h2 | stream | 10 KiB | 10 | 7929.39 | 8079.38 | +1.89 | 1.34 | 0.72 |
| wreq (blocking MT) | h2 | stream | 10 KiB | 50 | 7253.61 | 7241.27 | -0.17 | 1.54 | 1.43 |
| wreq (blocking MT) | h2 | stream | 10 KiB | 100 | 6805.00 | 6579.97 | -3.31 | 2.71 | 0.83 |
| wreq (blocking MT) | h2 | stream | 10 KiB | 150 | 6262.65 | 5998.76 | -4.21 | 1.84 | 1.52 |
| wreq (blocking MT) | h2 | stream | 64 KiB | 10 | 3669.80 | 3821.19 | +4.13 | 2.74 | 0.78 |
| wreq (blocking MT) | h2 | stream | 64 KiB | 50 | 3583.47 | 3672.85 | +2.49 | 0.57 | 0.37 |
| wreq (blocking MT) | h2 | stream | 64 KiB | 100 | 3425.93 | 3473.52 | +1.39 | 0.42 | 0.99 |
| wreq (blocking MT) | h2 | stream | 64 KiB | 150 | 3260.55 | 3365.22 | +3.21 | 1.37 | 0.82 |
| wreq (blocking MT) | h2 | stream | 128 KiB | 10 | 3562.94 | 3646.20 | +2.34 | 0.45 | 1.63 |
| wreq (blocking MT) | h2 | stream | 128 KiB | 50 | 3411.02 | 3567.04 | +4.57 | 2.38 | 0.75 |
| wreq (blocking MT) | h2 | stream | 128 KiB | 100 | 3341.69 | 3392.74 | +1.53 | 1.22 | 0.58 |
| wreq (blocking MT) | h2 | stream | 128 KiB | 150 | 3253.83 | 3273.59 | +0.61 | 0.76 | 0.93 |
| wreq (blocking MT) | h2 | stream | 1 MiB | 10 | 1017.82 | 1066.56 | +4.79 | 0.78 | 1.01 |
| wreq (blocking MT) | h2 | stream | 1 MiB | 50 | 1000.25 | 1048.21 | +4.79 | 1.95 | 0.43 |
| wreq (blocking MT) | h2 | stream | 1 MiB | 100 | 965.56 | 1007.23 | +4.32 | 0.81 | 1.55 |
| wreq (blocking MT) | h2 | stream | 1 MiB | 150 | 961.13 | 994.17 | +3.44 | 0.60 | 0.53 |
| wreq (blocking MT) | h2 | stream | 2 MiB | 10 | 771.04 | 815.87 | +5.81 | 0.20 | 1.18 |
| wreq (blocking MT) | h2 | stream | 2 MiB | 50 | 769.55 | 798.75 | +3.79 | 0.45 | 0.61 |
| wreq (blocking MT) | h2 | stream | 2 MiB | 100 | 747.33 | 766.36 | +2.55 | 0.32 | 0.31 |
| wreq (blocking MT) | h2 | stream | 2 MiB | 150 | 735.76 | 749.78 | +1.91 | 0.30 | 1.03 |
| wreq (blocking MT) | h2 | stream | 4 MiB | 10 | 529.07 | 546.02 | +3.20 | 0.17 | 0.22 |
| wreq (blocking MT) | h2 | stream | 4 MiB | 50 | 526.48 | 530.02 | +0.67 | 0.46 | 0.43 |
| wreq (blocking MT) | h2 | stream | 4 MiB | 100 | 502.13 | 506.97 | +0.96 | 0.35 | 0.93 |
| wreq (blocking MT) | h2 | stream | 4 MiB | 150 | 497.24 | 509.19 | +2.40 | 1.11 | 0.53 |
