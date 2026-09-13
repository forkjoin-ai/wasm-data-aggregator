# @affectively/wasm-data-aggregator

## Purpose, architecture, interface, operations, and failure boundaries

The purpose of this WASM architecture is deterministic bounded aggregation. Input schemas, output schemas, memory ceilings, and aggregation configuration form the contract. Test empty, malformed, large, and non-finite datasets. Allocation or decode failure rejects rather than emits a partial aggregate. Datasets are untrusted and results do not establish causation.

High-performance WebAssembly data aggregation utilities written in Rust.

[![npm](https://img.shields.io/npm/v/@affectively/wasm-data-aggregator.svg)](https://www.npmjs.com/package/@affectively/wasm-data-aggregator)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Features

- **Statistics** - Mean, median, std dev, percentiles
- **Grouping** - Group-by operations with aggregations
- **Time Series** - Rolling windows, resampling
- **Histogram** - Binning and distribution analysis

## Installation

```bash
npm install @affectively/wasm-data-aggregator
```text

## Quick Start

```typescript
import init, { compute_stats, group_by, rolling_mean } from '@affectively/wasm-data-aggregator';

await init();

const stats = compute_stats(values); // { mean, median, stdDev, min, max }
const grouped = group_by(data, 'category', 'sum');
const smoothed = rolling_mean(timeSeries, 7); // 7-point rolling mean
```

## License

MIT License

---

Made with ️ by [AFFECTIVELY](https://affectively.ai)
