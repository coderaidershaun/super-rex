---
name: fx-validation-plan
description: Expected traces of each flow and failure, written in the plan before code.
when_to_use: Use when a plan needs the expected trace of a flow.
metadata:
  kind: rule
  category: design
  claims:
    artefact: [plan]
  placement: open-when
  intents:
    - write the expected trace of a flow
    - list failure scenarios before code
    - say how a test proves the order
---
# Validation plan

Write each step that a test must see, in order.
