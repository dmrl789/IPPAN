# IPPAN AI TRAINING RULES

- Offline training scripts under ai_training/ may use Python floats.

- Runtime IPPAN must never train; runtime must only do deterministic integer inference from a frozen model.

- Large datasets must not be committed to the repo unless explicitly required and tracked with Git LFS.

