# Keep Instant Replay On

Enable Nvidia ShadowPlay Instant Replay when it isn't.

```txt
> .\keep-instant-replay-on.exe

2025-05-20T22:58:12.530069Z  INFO keep_instant_replay_on: Attempting to retrieve Nvidia HTTP server port and secret from: "{8BA1E16C-FC54-4595-9782-E370A5FBE8DA}"...
2025-05-20T22:58:12.530159Z DEBUG keep_instant_replay_on: Attempting to open file mapping...
2025-05-20T22:58:12.530260Z DEBUG keep_instant_replay_on: Attempting to map view of file...
2025-05-20T22:58:12.530346Z DEBUG keep_instant_replay_on: Attempting to query map view memory information...
2025-05-20T22:58:12.530433Z DEBUG keep_instant_replay_on: Attempting to read map view contents...
2025-05-20T22:58:12.531029Z DEBUG keep_instant_replay_on: Attempting to convert map view contents to CStr...
2025-05-20T22:58:12.531113Z DEBUG keep_instant_replay_on: Attempting to unmap view of file...
2025-05-20T22:58:12.531193Z DEBUG keep_instant_replay_on: Attempting to close file map handle...
2025-05-20T22:58:12.531306Z DEBUG keep_instant_replay_on: LpContents { port: 49952, secret: [REDACTED] }
2025-05-20T22:58:12.531372Z  INFO keep_instant_replay_on: Attempting to get Instant Replay status...
2025-05-20T22:58:12.531438Z DEBUG keep_instant_replay_on: Attempting to send HTTP GET request to Instant Replay status endpoint...
2025-05-20T22:58:14.546191Z  INFO keep_instant_replay_on: Attempting to enable Instant Replay...
2025-05-20T22:58:14.546332Z DEBUG keep_instant_replay_on: Attempting to send HTTP POST request to Instant Replay endpoint...
2025-05-20T22:58:16.849748Z  INFO keep_instant_replay_on: Instant Replay has been enabled
```

Run `.\keep-instant-replay-on.exe --help` for options, e.g.:

```txt
Usage: keep-instant-replay-on.exe [-s <seconds-between-checks>] [--file-mapping-uuid <file-mapping-uuid>] [--max-region-size <max-region-size>] [--expected-contents-size <expected-contents-size>] [--enable-endpoint <enable-endpoint>] [--log-path <log-path>]

Enable Nvidia ShadowPlay Instant Replay when it isn't.

Options:
  -s, --seconds-between-checks
                    how many seconds to wait between checking if Instant Replay
                    is enabled (default: 5)
  --file-mapping-uuid
                    UUID of the memory mapped file with the Nvidia HTTP server
                    port and secret (default:
                    {8BA1E16C-FC54-4595-9782-E370A5FBE8DA})
  --max-region-size maximum allowed region usize of the view of the memory
                    mapped file (default: 4096)
  --expected-contents-size
                    expected usize of the memory mapped file's contents
                    (default: 58)
  --enable-endpoint endpoint for enabling Instant Replay (default:
                    "/ShadowPlay/v.1.0/InstantReplay/Enable")
  --log-path        path to the log file (default: "{CARGO_CRATE_NAME}.log")
  --help, help      display usage information
```

## License

This project is licensed under the [GNU GPL v3.0](./LICENSE).

Made using the following resources:

| Resource                          | License                   |
|:---------------------------------:|:-------------------------:|
| [`NvShadowPlayAPI.cs`][better]    | [MIT][better-license]     |
| [`fixer.c`][shadow]               | [GPL-3.0][shadow-license] |
| [`DisableNVSP.cpp`][fivem]        | [LGPL-2.0][fivem-license] |

[better]: https://github.com/AJpon/BetterExperience/blob/1fb1d937c1e6381338217adc210d883d3d7101ab/NvNodeApi/Api/NvShadowPlayAPI.cs
[better-license]: https://github.com/AJpon/BetterExperience/blob/1fb1d937c1e6381338217adc210d883d3d7101ab/LICENSE
[shadow]: https://github.com/Verpous/AlwaysShadow/blob/daadaae597b976caf9526d888b1e33f24cf831e1/src/fixer.c
[shadow-license]: https://github.com/Verpous/AlwaysShadow/blob/daadaae597b976caf9526d888b1e33f24cf831e1/LICENSE
[fivem]: https://github.com/citizenfx/fivem/blob/c18e4725306ab344728958708bc0f575400d0f7c/code/client/launcher/DisableNVSP.cpp
[fivem-license]: https://github.com/citizenfx/fivem/blob/c18e4725306ab344728958708bc0f575400d0f7c/code/LICENSE
