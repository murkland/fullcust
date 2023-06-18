import { Solution, solve } from "./solver";

let it: Iterator<Solution> | null = null;

self.onmessage = function (e) {
    console.time(e.data.type);
    switch (e.data.type) {
        case "init": {
            const { parts, requirements, gridSettings, spinnableColors } =
                e.data.args;
            it = solve(parts, requirements, gridSettings, spinnableColors)[
                Symbol.iterator
            ]();
            break;
        }

        case "next": {
            const r = it!.next();
            self.postMessage({ type: "next", ...r });
            break;
        }
    }
    console.timeEnd(e.data.type);
};

self.postMessage({ type: "ready" });
