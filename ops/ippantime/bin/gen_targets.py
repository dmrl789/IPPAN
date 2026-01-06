#!/usr/bin/env python3
import sys, json

def main():
    import yaml  # PyYAML required
    inv_path = sys.argv[1] if len(sys.argv) > 1 else "ops/ippantime/inventory.yaml"
    with open(inv_path, "r", encoding="utf-8") as f:
        inv = yaml.safe_load(f)

    if inv.get("explicit_targets"):
        targets = [{"node": t["node"], "url": t["url"]} for t in inv["explicit_targets"]]
    else:
        base_port = int(inv["http"]["base_port"])
        targets = []
        for s in inv["servers"]:
            host = s["host"]
            a, b = s["node_index_range"]
            for i in range(int(a), int(b) + 1):
                targets.append({"node": f"node-{i:02d}", "url": f"http://{host}:{base_port + i}"})

    print(json.dumps(targets, indent=2))

if __name__ == "__main__":
    main()
