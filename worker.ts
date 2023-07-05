import { GridSettings, Part, Requirement, Solution, solve } from "./solver";

export type EventData =
    | { type: "next" }
    | {
          type: "init";
          args: {
              parts: Part[];
              requirements: Requirement[];
              gridSettings: GridSettings;
              spinnableColors: boolean[];
          };
      };

let it: Iterator<Solution> | null = null;

self.onmessage = function (e: MessageEvent<EventData>) {
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
