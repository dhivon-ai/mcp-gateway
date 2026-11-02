# License and ownership

Dhivon MCP Gateway is a product of Dhivon AI.

Copyright 2026 Dhivon AI. The project's own code is licensed under the
[Apache License, Version 2.0](../LICENSE). The [project NOTICE](../NOTICE)
records ownership and bundled attribution.

Third-party components retain their respective copyrights and licenses.
The bundled Manrope font remains under the SIL Open Font License 1.1. Its
[original copyright notice and full license](../assets/brand/fonts/OFL.txt)
are distributed unchanged.

## Finding notices in a build

| Delivery       | Project license and ownership                            | Manrope copyright and full OFL                               |
| -------------- | -------------------------------------------------------- | ------------------------------------------------------------ |
| Source         | Root `LICENSE` and `NOTICE`                              | `assets/brand/fonts/OFL.txt`                                 |
| Mac archive    | `LICENSE` and `NOTICE` beside the executable             | `licenses/Manrope-OFL.txt`                                   |
| Service image  | `/usr/share/doc/dhivon-mcp-gateway/LICENSE` and `NOTICE` | `/usr/share/doc/dhivon-mcp-gateway/licenses/Manrope-OFL.txt` |
| Public website | Linked license and notice above                          | Linked font notice above                                     |

Maintainers verify the emitted files against source bytes with
`node scripts/check-notices.ts --archive <archive> --image <image> --site <site-directory>`.
Each option can also be run separately. Mac packaging, the deployment rehearsal
and website CI run these checks for their respective outputs.
