#!/usr/bin/env bash
set -euo pipefail
python3 -m pip install --no-index --find-links ai_assets/vendor/pip_wheels -r ai_training/requirements.lock.txt

