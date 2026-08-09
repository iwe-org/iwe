'use strict'

const { execFileSync } = require('child_process')

const PLATFORM_PACKAGES = {
  'darwin-arm64': '@iwe-org/cli-darwin-arm64',
  'darwin-x64': '@iwe-org/cli-darwin-x64',
  'linux-x64': '@iwe-org/cli-linux-x64',
  'linux-arm64': '@iwe-org/cli-linux-arm64',
  'win32-x64': '@iwe-org/cli-win32-x64'
}

function binaryPath(name) {
  const key = process.platform + '-' + process.arch
  const pkg = PLATFORM_PACKAGES[key]
  if (!pkg) {
    console.error('iwe: unsupported platform: ' + key)
    console.error('iwe: supported platforms: ' + Object.keys(PLATFORM_PACKAGES).join(', '))
    console.error('iwe: see https://iwe.md for other install options')
    process.exit(1)
  }
  const file = process.platform === 'win32' ? name + '.exe' : name
  try {
    return require.resolve(pkg + '/' + file)
  } catch (err) {
    console.error('iwe: platform package ' + pkg + ' is not installed')
    console.error('iwe: npm may have skipped optionalDependencies; try reinstalling without --no-optional')
    process.exit(1)
  }
}

function run(name) {
  const bin = binaryPath(name)
  try {
    execFileSync(bin, process.argv.slice(2), { stdio: 'inherit' })
  } catch (err) {
    if (typeof err.status === 'number') process.exit(err.status)
    if (err.signal) process.exit(1)
    throw err
  }
}

module.exports = { run, binaryPath }
