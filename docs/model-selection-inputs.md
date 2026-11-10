# Model Selection Function Inputs

| Input | Fast / Low | Medium | Long / High | `None` |
|---|---|---|---|---|
| **Duration** | **Fast:** Finish quickly. | A moderate wait is acceptable. | **Long:** A longer run is acceptable. | Not relevant. Ignored. |
| **Budget** | **Low:** Keep spending low. | Allow moderate spending. | **High:** Allow higher spending when useful. | Not relevant. Ignored. |
| **Intelligence** | **Low:** Basic capability is sufficient for routine tasks. | Needs reasoning and judgment. | **High:** Needs advanced reasoning for difficult tasks. | Not relevant. Ignored. |
| **Autonomy** | **Low:** Frequent guidance is acceptable. | Occasional help is acceptable. | **High:** Work independently; ask only when necessary. | Not relevant. Ignored. |
| **Task Required Reliability** | **Low:** Occasional failures are acceptable. | Failures should be uncommon. | **High:** Failures must be very rare. | Not relevant. Ignored. |
| **Task Failure Consequence** | **Low:** Mistakes cause minor, easily reversible problems. | Mistakes cause meaningful rework or disruption. | **High:** Mistakes cause serious damage or losses that are difficult to reverse. | Not relevant. Ignored. |
| **Task Ease of Verification** | **Low:** Correctness is difficult to check. | Checking requires some review or testing. | **High:** Quick, reliable checks are available. | Not relevant. Ignored. |

`None` means **the input is not relevant and is ignored**. OMS infers nothing from the task. A higher budget or longer duration permits additional spending or time; it does not require it.
