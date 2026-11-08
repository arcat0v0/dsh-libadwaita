import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import { captureConsoleDiagnostics, DiagnosticsBuffer } from '../../src/diagnostics.js';

describe('DiagnosticsBuffer', () => {
  it('keeps the most recent entries within the limit', () => {
    const buffer = new DiagnosticsBuffer(3);
    buffer.push('one');
    buffer.push('two');
    buffer.push('three');
    buffer.push('four');
    assert.deepEqual(buffer.list(), ['two', 'three', 'four']);
  });

  it('trims oversized lines and drops empties', () => {
    const buffer = new DiagnosticsBuffer(5);
    buffer.push('   ');
    buffer.push('x'.repeat(600));
    const entries = buffer.list();
    assert.equal(entries.length, 1);
    assert.ok(entries[0].length <= 500);
  });

  it('captures console warn and error through the capture hook', () => {
    const buffer = new DiagnosticsBuffer(10);
    const restore = captureConsoleDiagnostics(buffer);
    try {
      console.warn('plugin alpha failed to start');
      console.error('agent loop crashed');
      const entries = buffer.list();
      assert.ok(entries.some((entry) => entry.includes('plugin alpha failed to start')));
      assert.ok(entries.some((entry) => entry.includes('agent loop crashed')));
    } finally {
      restore();
    }
  });
});
