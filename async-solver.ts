import { GridSettings, Part, Requirement, Solution, solve } from "./solver";

export default class AsyncSolver {
    worker: Worker;
    it: AsyncIterator<Solution>;

    constructor(
        parts: Part[],
        requirements: Requirement[],
        gridSettings: GridSettings,
        spinnableColors: boolean[]
    ) {
        const worker = new Worker(
            new URL("./async-solver.ts", import.meta.url),
            {
                type: "module",
            }
        );
        this.worker = worker;
        const args = { parts, requirements, gridSettings, spinnableColors };

        this.it = (async function* () {
            const ready = await new Promise<{ type: string }>((resolve) => {
                worker.addEventListener("message", function eh(msg) {
                    worker.removeEventListener("message", eh);
                    resolve(msg.data);
                });
            });
            if (ready.type != "ready") {
                throw "not ready";
            }

            worker.postMessage({
                type: "init",
                args,
            });

            while (true) {
                const solution = await new Promise<{
                    value: Solution;
                    done: boolean;
                }>((resolve) => {
                    worker.addEventListener("message", function eh(msg) {
                        worker.removeEventListener("message", eh);
                        resolve(msg.data);
                    });
                    worker.postMessage({ type: "next" });
                });
                if (solution.done) {
                    break;
                }
                yield solution.value;
            }
        })();
    }

    next() {
        return this.it.next();
    }

    [Symbol.asyncIterator]() {
        return this.it;
    }

    kill() {
        this.worker.terminate();
    }
}

declare var WorkerGlobalScope: any;
if (
    typeof WorkerGlobalScope !== "undefined" &&
    self instanceof WorkerGlobalScope
) {
    let it: Iterator<Solution> | null = null;

    self.onmessage = function (
        e: MessageEvent<
            | { type: "next" }
            | {
                  type: "init";
                  args: {
                      parts: Part[];
                      requirements: Requirement[];
                      gridSettings: GridSettings;
                      spinnableColors: boolean[];
                  };
              }
        >
    ) {
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
}
