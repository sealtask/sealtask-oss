import assert from 'node:assert/strict'
import {
  mkdtemp,
  mkdir,
  rm,
  writeFile,
} from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import test from 'node:test'

import { checkPackageBoundary } from './check-boundary.mjs'

test('checks non-TypeScript published files without rejecting live CBOR or exclusion docs', async (context) => {
  const packageRoot = await createPackageFixture()
  context.after(() =>
    rm(packageRoot, {
      force: true,
      recursive: true,
    }),
  )

  await writeFile(
    join(packageRoot, 'README.md'),
    'The legacy-CBOR decoder and fixtures are intentionally excluded.\n',
  )
  await writeFile(
    join(packageRoot, 'src', 'live-cbor.js'),
    "import { decode } from 'cbor-x'\nexport { decode }\n",
  )
  await writeFile(
    join(packageRoot, 'src', 'fixtures.json'),
    '{"format":"current-cbor"}\n',
  )

  assert.deepEqual(
    await checkPackageBoundary(packageRoot),
    [],
  )

  await writeFile(
    join(packageRoot, 'src', 'fixtures.json'),
    '{"format":"legacy-cbor"}\n',
  )

  assert.match(
    (await checkPackageBoundary(packageRoot)).join('\n'),
    /src\/fixtures\.json: contains private legacy-CBOR compatibility code or fixture data/,
  )

  await writeFile(
    join(packageRoot, 'src', 'fixture-notes.md'),
    'Embedded legacy-CBOR fixture data.\n',
  )

  assert.match(
    (await checkPackageBoundary(packageRoot)).join('\n'),
    /src\/fixture-notes\.md: contains private legacy-CBOR compatibility code or fixture data/,
  )
})

test('rejects legacy-CBOR fixture data embedded in published README code', async (context) => {
  const packageRoot = await createPackageFixture()
  context.after(() =>
    rm(packageRoot, {
      force: true,
      recursive: true,
    }),
  )

  await writeFile(
    join(packageRoot, 'README.md'),
    [
      'Legacy-CBOR support remains intentionally excluded.',
      '',
      '```json',
      '{"format":"legacy-cbor"}',
      '```',
      '',
    ].join('\n'),
  )

  assert.match(
    (await checkPackageBoundary(packageRoot)).join('\n'),
    /README\.md: contains private legacy-CBOR compatibility code or fixture data/,
  )
})

test('rejects a published legacy-CBOR artifact even when its bytes are opaque', async (context) => {
  const packageRoot = await createPackageFixture()
  context.after(() =>
    rm(packageRoot, {
      force: true,
      recursive: true,
    }),
  )

  await writeFile(
    join(packageRoot, 'src', 'legacy-cbor-fixture.bin'),
    new Uint8Array([0xa1, 0x01, 0x02]),
  )

  assert.match(
    (await checkPackageBoundary(packageRoot)).join('\n'),
    /src\/legacy-cbor-fixture\.bin: exposes a private legacy-CBOR artifact in the public package tree/,
  )
})

test('rejects a legacy-CBOR artifact in the public Git test tree', async (context) => {
  const packageRoot = await createPackageFixture()
  context.after(() =>
    rm(packageRoot, {
      force: true,
      recursive: true,
    }),
  )
  await mkdir(join(packageRoot, 'test', 'fixtures'), {
    recursive: true,
  })
  await writeFile(
    join(packageRoot, 'test', 'fixtures', 'legacy-cbor.bin'),
    new Uint8Array([0xa1, 0x01, 0x02]),
  )

  assert.match(
    (await checkPackageBoundary(packageRoot)).join('\n'),
    /test\/fixtures\/legacy-cbor\.bin: exposes a private legacy-CBOR artifact in the public package tree/,
  )
})

async function createPackageFixture() {
  const packageRoot = await mkdtemp(
    join(tmpdir(), 'crypto-web-boundary-'),
  )
  await mkdir(join(packageRoot, 'src'))
  await writeFile(
    join(packageRoot, 'package.json'),
    JSON.stringify({
      files: [
        'src',
        'README.md',
      ],
    }),
  )
  await writeFile(join(packageRoot, 'README.md'), '# Test package\n')
  return packageRoot
}
