const path = require("path");
const HtmlWebpackPlugin = require("html-webpack-plugin");
const webpack = require("webpack");
const WasmPackPlugin = require("@wasm-tool/wasm-pack-plugin");

module.exports = {
  entry: "./index.js",
  output: {
    path: path.resolve(__dirname, "..", "dist", "console_log"),
    filename: "index.js",
  },
  plugins: [
    new HtmlWebpackPlugin({ template: "./index.html" }),
    new WasmPackPlugin({
      crateDirectory: __dirname,
      extraArgs: "--scope 1io",
    }),
  ],
  mode: "development",
  experiments: {
    asyncWebAssembly: true,
  },
};
