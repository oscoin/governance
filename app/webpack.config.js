const path = require('path');
const CopyPlugin = require('copy-webpack-plugin');
const { CleanWebpackPlugin } = require('clean-webpack-plugin');

const outputDir = path.join(__dirname, 'build/');
const assetsDir = path.join(__dirname, 'assets/');

const isProd = process.env.NODE_ENV === 'production';

module.exports = {
  entry: './lib/js/src/renderer/Index.bs.js',
  mode: isProd ? 'production' : 'development',
  output: {
    path: outputDir,
    filename: 'index.js'
  },
  plugins: [
    new CleanWebpackPlugin(),
    new CopyPlugin([
      { from: 'assets', to: './' }
    ])
  ],
  devServer: {
    compress: true,
    contentBase: [outputDir, assetsDir],
    port: process.env.PORT || 8000,
    historyApiFallback: true
  }
};
