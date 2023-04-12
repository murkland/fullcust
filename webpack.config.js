const path = require("path");
const CopyPlugin = require("copy-webpack-plugin");
const WasmPackPlugin = require("@wasm-tool/wasm-pack-plugin");

const dist = path.resolve(__dirname, "dist");

module.exports = {
    mode: "production",
    entry: {
        index: "./js/index.ts",
    },
    module: {
        rules: [
            {
                test: /\.tsx?$/,
                use: "ts-loader",
                exclude: /node_modules/,
            },
        ],
    },
    resolve: {
        extensions: [".tsx", ".ts", ".js"],
    },
    output: {
        path: dist,
        filename: "[name].js",
    },
    devServer: {
        static: dist,
    },
    plugins: [
        new CopyPlugin({ patterns: [path.resolve(__dirname, "static")] }),

        new WasmPackPlugin({
            crateDirectory: __dirname,
        }),
    ],
    experiments: {
        asyncWebAssembly: true,
    },
};
