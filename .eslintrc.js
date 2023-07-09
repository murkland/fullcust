module.exports = {
    env: {
        browser: true,
        es2021: true,
    },
    extends: [
        "defaults",
        "plugin:react/recommended",
        "plugin:react-hooks/recommended",
    ],
    overrides: [
        {
            env: {
                node: true,
            },
            files: [".eslintrc.{js,cjs}"],
            parserOptions: {
                sourceType: "script",
            },
        },
    ],
    parserOptions: {
        ecmaVersion: "latest",
        sourceType: "module",
        project: "./tsconfig.json",
    },
    plugins: ["react", "react-hooks"],
    rules: {
        "comma-dangle": ["error", "always-multiline"],
        "react-hooks/rules-of-hooks": "error",
        "react-hooks/exhaustive-deps": "error",
    },
};
