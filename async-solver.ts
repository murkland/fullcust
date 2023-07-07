import { GridSettings, Part, Requirement, Solution } from "./solver";

import type { Request, Response } from "./worker";

export default class AsyncSolver {
    worker: Worker;
    it: AsyncIterator<Solution>;

    constructor(
        parts: Part[],
        requirements: Requirement[],
        gridSettings: GridSettings,
        spinnableColors: boolean[]
    ) {
        const worker = new Worker(new URL("./worker.ts", import.meta.url), {
            type: "module",
        });
        this.worker = worker;

        this.it = (async function* () {
            {
                const e = await new Promise<MessageEvent<Response>>(
                    (resolve) => {
                        worker.addEventListener("message", function eh(e) {
                            worker.removeEventListener("message", eh);
                            resolve(e);
                        });
                    }
                );
                if (e.data.type != "ready") {
                    throw "not ready";
                }
            }

            worker.postMessage({
                type: "init",
                args: { parts, requirements, gridSettings, spinnableColors },
            } as Request);

            while (true) {
                const e = await new Promise<MessageEvent<Response>>(
                    (resolve) => {
                        worker.addEventListener("message", function eh(e) {
                            worker.removeEventListener("message", eh);
                            resolve(e);
                        });
                        worker.postMessage({ type: "next" } as Request);
                    }
                );
                if (e.data.type != "next") {
                    throw "not ready";
                }
                if (e.data.done) {
                    break;
                }
                yield e.data.value;
            }
        })();
    }

    next() {
        return this.it.next();
    }

    terminate() {
        this.worker.terminate();
    }
}
