'use strict'

const fs = require('fs')
const path = require('path')

const version = process.argv[2]
if (!version || !/^\d+\.\d+\.\d+/.test(version)) {
  console.error('usage: node stamp.js <version>')
  process.exit(1)
}

const packages = [
  'cli-darwin-arm64',
  'cli-darwin-x64',
  'cli-linux-x64',
  'cli-linux-arm64',
  'cli-win32-x64',
  'iwe',
  'mcp'
]

for (const dir of packages) {
  const file = path.join(__dirname, dir, 'package.json')
  const pkg = JSON.parse(fs.readFileSync(file, 'utf8'))
  pkg.version = version
  if (pkg.optionalDependencies) {
    for (const dep of Object.keys(pkg.optionalDependencies)) {
      pkg.optionalDependencies[dep] = version
    }
  }
  if (pkg.dependencies && pkg.dependencies['@iwe-org/iwe']) {
    pkg.dependencies['@iwe-org/iwe'] = version
  }
  fs.writeFileSync(file, JSON.stringify(pkg, null, 2) + '\n')
  console.log(pkg.name + '@' + version)
}

const serverFile = path.join(__dirname, '..', 'server.json')
const server = JSON.parse(fs.readFileSync(serverFile, 'utf8'))
server.version = version
for (const pkg of server.packages) {
  pkg.version = version
}
fs.writeFileSync(serverFile, JSON.stringify(server, null, 2) + '\n')
console.log(server.name + '@' + version)
