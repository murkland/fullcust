import { GridSettings, Part, Requirement, Solution } from "./solver";

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
