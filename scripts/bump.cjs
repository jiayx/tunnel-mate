const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

const newVersion = process.argv[2];
if (!newVersion) {
  console.error('请指定版本号，例如: pnpm bump 1.0.0');
  process.exit(1);
}

// 验证版本号格式 (x.y.z)
if (!/^\d+\.\d+\.\d+(-\w+(\.\d+)?)?$/.test(newVersion)) {
  console.error('错误: 版本号格式必须符合 SemVer 规范 (例如 1.0.0 或 1.0.0-beta.1)');
  process.exit(1);
}

try {
  // 1. 更新 package.json
  const pkgPath = path.resolve(__dirname, '../package.json');
  const pkg = JSON.parse(fs.readFileSync(pkgPath, 'utf8'));
  pkg.version = newVersion;
  fs.writeFileSync(pkgPath, JSON.stringify(pkg, null, 2) + '\n');
  console.log(`✓ 成功更新 package.json 为 ${newVersion}`);

  // 2. 更新 tauri.conf.json
  const tauriPath = path.resolve(__dirname, '../src-tauri/tauri.conf.json');
  const tauri = JSON.parse(fs.readFileSync(tauriPath, 'utf8'));
  tauri.version = newVersion;
  fs.writeFileSync(tauriPath, JSON.stringify(tauri, null, 2) + '\n');
  console.log(`✓ 成功更新 tauri.conf.json 为 ${newVersion}`);

  // 3. 更新 Cargo.toml
  const cargoPath = path.resolve(__dirname, '../src-tauri/Cargo.toml');
  let cargo = fs.readFileSync(cargoPath, 'utf8');
  cargo = cargo.replace(/^version = "[^"]*"/m, `version = "${newVersion}"`);
  fs.writeFileSync(cargoPath, cargo);
  console.log(`✓ 成功更新 Cargo.toml 为 ${newVersion}`);

  console.log('\n📦 开始执行 Git 提交、打 Tag 和推送...');

  // 4. Git 提交和推送
  const commitMsg = `chore: release v${newVersion}`;
  const tagName = `v${newVersion}`;

  console.log('- 正在暂存修改的文件...');
  execSync('git add package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml', { stdio: 'inherit' });

  console.log(`- 正在提交: "${commitMsg}"...`);
  execSync(`git commit -m "${commitMsg}"`, { stdio: 'inherit' });

  console.log(`- 正在创建本地 Tag: ${tagName}...`);
  execSync(`git tag ${tagName}`, { stdio: 'inherit' });

  console.log('- 正在推送代码和 Tag 到远程仓库...');
  execSync('git push origin HEAD --tags', { stdio: 'inherit' });

  console.log(`\n🎉 版本号已成功更新为: ${newVersion}，并已成功推送至 GitHub！`);
} catch (error) {
  console.error('\n❌ 操作失败:', error.message);
  process.exit(1);
}
