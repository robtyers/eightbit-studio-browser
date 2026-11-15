// Exact portable workbench graph retained in the public browser source archive.
const components = ['WorkbenchShell', 'WorkbenchResizeHandle', 'WorkbenchPanelTabs', 'EditorToolbar', 'EditorTabs', 'Icons', 'WelcomeView', 'ProcessorLogo', 'MonitorConsole', 'BasicDebugPanel', 'MachineDebugPanel'];
const services = ['recentProjects', 'editorCommands', 'workbenchLayout', 'monitorProtocol', 'monitorFrame', 'originalBasicDebug', 'developmentHost', 'originalRuntime', 'originalMachineDebug'];
export const browserSharedFiles = [
  ...components.map(name => `packages/workbench/src/components/${name}.tsx`),
  ...services.map(name => `packages/workbench/src/services/${name}.ts`),
  'packages/workbench/src/components/MonitorAudio.ts', 'packages/workbench/src/hooks/useWorkbenchCommands.ts',
  'packages/workbench/src/services/commands.json', 'packages/workbench/src/styles.css', 'packages/workbench/src/macos.css',
  'packages/workbench/branding/fonts/Silkscreen-Regular.ttf', 'packages/workbench/branding/fonts/OFL.txt',
  'packages/workbench/branding/cpu.svg', 'packages/workbench/src/assets.d.ts',
];
export function browserSharedExport(name) {
  return name === './styles.css' || name === './macos.css' || name === './hooks/useWorkbenchCommands'
    || services.some(service => name === `./services/${service}`)
    || components.some(component => name === `./components/${component}`);
}

// The expanded profile retains canonical source only. Overflow qualification
// images/tests remain outside the original-firmware distribution exception.
export function excludedBrowserSource(path) {
  const complete = 'families/dragon/firmware/basic-complete';
  if (path === complete || path.startsWith(`${complete}/`)) {
    const name = path.slice(complete.length + 1);
    return !['', 'source', 'fixed-layout.asm', 'layout.asm'].includes(name) && !/^source\/[a-z0-9-]+\.inc$/.test(name);
  }
  return [
    'families/dragon/crates/eightbit-dragon-emulator/tests/basic_complete.rs',
    'families/dragon/crates/eightbit-dragon-emulator/tests/fixtures/complete-basic',
  ].some(excluded => path === excluded || path.startsWith(`${excluded}/`));
}
