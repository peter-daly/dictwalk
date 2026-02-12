# Benchmarks

## 2026-02-12T17:06:11+00:00

**Description:** Re-run after map-fusion optimization to validate stability (same machine, immediate repeat)

- Python: `3.13.11`
- Repeats: `7`
- Warmup: `2`

### parse-heavy: get_unique_paths

| backend | mean (ms) | median (ms) | stdev (ms) | calls |    ops/s |
| ------- | --------: | ----------: | ---------: | ----: | -------: |
| python  |     3.760 |       3.759 |      0.095 |   500 | 132974.3 |
| rust    |     1.313 |       1.311 |      0.073 |   500 | 380747.0 |

Speedup (rust vs python): `2.86x`

### read-heavy: get_repeated_filter_path

| backend | mean (ms) | median (ms) | stdev (ms) | calls |   ops/s |
| ------- | --------: | ----------: | ---------: | ----: | ------: |
| python  |   384.984 |     380.771 |     11.815 |   300 |   779.3 |
| rust    |    17.971 |      17.681 |      0.863 |   300 | 16693.2 |

Speedup (rust vs python): `21.42x`

### read-heavy: get_repeated_builtin_predicate_filter_path

| backend | mean (ms) | median (ms) | stdev (ms) | calls |  ops/s |
| ------- | --------: | ----------: | ---------: | ----: | -----: |
| python  |    40.795 |      40.786 |      0.247 |   300 | 7353.8 |
| rust    |    35.958 |      35.914 |      0.361 |   300 | 8343.2 |

Speedup (rust vs python): `1.13x`

### transform-heavy: get_numeric_output_pipeline

| backend | mean (ms) | median (ms) | stdev (ms) | calls |   ops/s |
| ------- | --------: | ----------: | ---------: | ----: | ------: |
| python  |   380.787 |     374.376 |     16.885 | 20000 | 52522.8 |
| rust    |   262.367 |     262.974 |      3.219 | 20000 | 76229.0 |

Speedup (rust vs python): `1.45x`

### transform-heavy: get_string_output_pipeline

| backend | mean (ms) | median (ms) | stdev (ms) | calls |   ops/s |
| ------- | --------: | ----------: | ---------: | ----: | ------: |
| python  |   364.073 |     360.994 |      6.794 | 20000 | 54934.1 |
| rust    |   274.541 |     273.285 |      2.605 | 20000 | 72848.9 |

Speedup (rust vs python): `1.33x`

### transform-heavy: get_deep_10_layer_filter_pipeline

| backend | mean (ms) | median (ms) | stdev (ms) | calls | ops/s |
| ------- | --------: | ----------: | ---------: | ----: | ----: |
| python  |   135.925 |     135.534 |      1.150 |   120 | 882.8 |
| rust    |   137.296 |     135.703 |      4.344 |   120 | 874.0 |

Speedup (rust vs python): `0.99x`

### write-heavy: set_filter_writes

| backend | mean (ms) | median (ms) | stdev (ms) | calls |  ops/s |
| ------- | --------: | ----------: | ---------: | ----: | -----: |
| python  |    58.236 |      57.768 |      1.616 |    40 |  686.9 |
| rust    |     6.928 |       6.807 |      0.407 |    40 | 5773.3 |

Speedup (rust vs python): `8.41x`

### write-heavy: unset_filter_writes

| backend | mean (ms) | median (ms) | stdev (ms) | calls |  ops/s |
| ------- | --------: | ----------: | ---------: | ----: | -----: |
| python  |    56.833 |      57.143 |      0.576 |    40 |  703.8 |
| rust    |     4.329 |       4.303 |      0.189 |    40 | 9239.4 |

Speedup (rust vs python): `13.13x`

## 2026-02-12T18:08:27+00:00

**Description:** Python wrapper only benchmark run (no direct rust backend row).

- Python: `3.13.11`
- Repeats: `7`
- Warmup: `2`

### parse-heavy: get_unique_paths

| backend        | mean (ms) | median (ms) | stdev (ms) | calls |   ops/s |
| -------------- | --------: | ----------: | ---------: | ----: | ------: |
| python-wrapper |     9.289 |       9.212 |      0.910 |   500 | 53825.6 |

### read-heavy: get_repeated_filter_path

| backend        | mean (ms) | median (ms) | stdev (ms) | calls |  ops/s |
| -------------- | --------: | ----------: | ---------: | ----: | -----: |
| python-wrapper |    65.592 |      64.686 |      1.932 |   300 | 4573.7 |

### read-heavy: get_repeated_builtin_predicate_filter_path

| backend        | mean (ms) | median (ms) | stdev (ms) | calls |  ops/s |
| -------------- | --------: | ----------: | ---------: | ----: | -----: |
| python-wrapper |   138.405 |     138.370 |      0.302 |   300 | 2167.5 |

### transform-heavy: get_numeric_output_pipeline

| backend        | mean (ms) | median (ms) | stdev (ms) | calls |   ops/s |
| -------------- | --------: | ----------: | ---------: | ----: | ------: |
| python-wrapper |   956.342 |     957.390 |     15.367 | 20000 | 20913.0 |

### transform-heavy: get_string_output_pipeline

| backend        | mean (ms) | median (ms) | stdev (ms) | calls |   ops/s |
| -------------- | --------: | ----------: | ---------: | ----: | ------: |
| python-wrapper |   957.292 |     938.537 |     58.974 | 20000 | 20892.3 |

### transform-heavy: get_deep_10_layer_filter_pipeline

| backend        | mean (ms) | median (ms) | stdev (ms) | calls | ops/s |
| -------------- | --------: | ----------: | ---------: | ----: | ----: |
| python-wrapper |   273.291 |     266.591 |     10.165 |   120 | 439.1 |

### write-heavy: set_filter_writes

| backend        | mean (ms) | median (ms) | stdev (ms) | calls |  ops/s |
| -------------- | --------: | ----------: | ---------: | ----: | -----: |
| python-wrapper |    19.423 |      19.408 |      0.356 |    40 | 2059.4 |

### write-heavy: unset_filter_writes

| backend        | mean (ms) | median (ms) | stdev (ms) | calls |  ops/s |
| -------------- | --------: | ----------: | ---------: | ----: | -----: |
| python-wrapper |    10.694 |      10.615 |      0.242 |    40 | 3740.4 |

## 2026-02-12T18:25:28+00:00

**Description:** Fresh benchmark after switching dictwalk module to direct PyO3 object

- Python: `3.13.11`
- Repeats: `7`
- Warmup: `2`

### parse-heavy: get_unique_paths

| backend | mean (ms) | median (ms) | stdev (ms) | calls | ops/s |
|---|---:|---:|---:|---:|---:|
| python | 8.336 | 8.289 | 0.114 | 500 | 59982.6 |
| rust | 8.338 | 8.351 | 0.068 | 500 | 59969.5 |

Speedup (rust vs python): `1.00x`

### read-heavy: get_repeated_filter_path

| backend | mean (ms) | median (ms) | stdev (ms) | calls | ops/s |
|---|---:|---:|---:|---:|---:|
| python | 66.108 | 65.483 | 1.243 | 300 | 4538.0 |
| rust | 65.226 | 64.995 | 0.416 | 300 | 4599.4 |

Speedup (rust vs python): `1.01x`

### read-heavy: get_repeated_builtin_predicate_filter_path

| backend | mean (ms) | median (ms) | stdev (ms) | calls | ops/s |
|---|---:|---:|---:|---:|---:|
| python | 146.671 | 144.146 | 12.430 | 300 | 2045.4 |
| rust | 144.743 | 140.384 | 11.293 | 300 | 2072.6 |

Speedup (rust vs python): `1.01x`

### transform-heavy: get_numeric_output_pipeline

| backend | mean (ms) | median (ms) | stdev (ms) | calls | ops/s |
|---|---:|---:|---:|---:|---:|
| python | 990.348 | 979.617 | 38.476 | 20000 | 20194.9 |
| rust | 962.790 | 959.571 | 16.288 | 20000 | 20773.0 |

Speedup (rust vs python): `1.03x`

### transform-heavy: get_string_output_pipeline

| backend | mean (ms) | median (ms) | stdev (ms) | calls | ops/s |
|---|---:|---:|---:|---:|---:|
| python | 920.394 | 917.414 | 6.316 | 20000 | 21729.8 |
| rust | 1017.535 | 916.979 | 197.500 | 20000 | 19655.3 |

Speedup (rust vs python): `0.90x`

### transform-heavy: get_deep_10_layer_filter_pipeline

| backend | mean (ms) | median (ms) | stdev (ms) | calls | ops/s |
|---|---:|---:|---:|---:|---:|
| python | 277.708 | 275.409 | 6.905 | 120 | 432.1 |
| rust | 271.593 | 271.076 | 2.392 | 120 | 441.8 |

Speedup (rust vs python): `1.02x`

### write-heavy: set_filter_writes

| backend | mean (ms) | median (ms) | stdev (ms) | calls | ops/s |
|---|---:|---:|---:|---:|---:|
| python | 18.719 | 18.618 | 0.620 | 40 | 2136.9 |
| rust | 19.013 | 18.886 | 0.388 | 40 | 2103.8 |

Speedup (rust vs python): `0.98x`

### write-heavy: unset_filter_writes

| backend | mean (ms) | median (ms) | stdev (ms) | calls | ops/s |
|---|---:|---:|---:|---:|---:|
| python | 10.518 | 10.416 | 0.475 | 40 | 3803.0 |
| rust | 10.466 | 10.432 | 0.447 | 40 | 3821.7 |

Speedup (rust vs python): `1.00x`

