import { spectrumFirmwareId, type SpectrumMachine } from "@eightbitstudio/workbench/services/monitorFrame";
import type { MachinePanelStatus } from "@eightbitstudio/workbench/components/MachineDebugPanel";
import { record } from "./contracts";
export interface SpectrumInspection extends MachinePanelStatus {sessionId:number;stopId:number;sourceLine:number|null;revision:string}
export interface SpectrumDebuggerPort {command(operation:"step"|"continue"|"pause",stopId:number):Promise<unknown>}
function integer(value:unknown,max:number):value is number {return typeof value==="number"&&Number.isSafeInteger(value)&&value>=0&&value<=max;}
export function spectrumInspection(value:unknown,sessionId:number,revision:string,target:SpectrumMachine="spectrum48"):SpectrumInspection|undefined {
 if(!record(value)||value.schemaVersion!==1||value.sessionId!==sessionId||value.firmwareId!==spectrumFirmwareId(target))throw new Error("Invalid Spectrum debugger owner.");
 if(value.state==="failed")return {sessionId,revision,state:"failed",stopId:0,valuesValid:false,breakpointVerified:false,breakpointAddress:0,sourceLine:null,registers:null,memory:null,disassembly:[],message:"Original Spectrum failed. Show Game to reset, or Stop and build again."};
 if(value.debug===null||value.state==="stopped")return undefined;
 const d=value.debug;
 if(!record(d)||d.revision!==revision||!integer(d.stopId,0xffffffff)||(d.state!=="running"&&d.state!=="stopped")||typeof d.valuesValid!=="boolean"||typeof d.breakpointVerified!=="boolean"||!integer(d.breakpointAddress,65535)||(d.sourceLine!==null&&(!integer(d.sourceLine,128*1024)||d.sourceLine===0)))throw new Error("Invalid Spectrum debugger inspection.");
 let registers:Record<string,number>|null=null;
 if(d.registers!==null){
  const r=d.registers;const names=["af","bc","de","hl","af'","bc'","de'","hl'","ix","iy","sp","pc"];
  if(!record(r)||Object.keys(r).length!==names.length)throw new Error("Invalid Spectrum registers.");
  registers={};for(const name of names){const v=r[name];if(!integer(v,65535))throw new Error("Invalid Spectrum register.");registers[name]=v;}
 }
 let memory:{startAddress:number;bytes:number[]}|null=null;
 if(d.memory!==null){const m=d.memory;if(!record(m)||!integer(m.startAddress,65472)||!Array.isArray(m.bytes)||m.bytes.length!==64||!m.bytes.every((n:unknown)=>integer(n,255)))throw new Error("Invalid Spectrum memory.");memory={startAddress:m.startAddress,bytes:m.bytes};}
 if(d.state==="running"&&(d.valuesValid||registers||memory||d.breakpointVerified)||d.state==="stopped"&&(!d.valuesValid||!registers||!memory))throw new Error("Stale Spectrum inspection values.");
 return {sessionId,revision,state:d.state,stopId:d.stopId,valuesValid:d.valuesValid,breakpointVerified:d.breakpointVerified,breakpointAddress:d.breakpointAddress,sourceLine:d.sourceLine,registers,memory,disassembly:[],message:d.state==="stopped"?"Original Spectrum paused":"Original Spectrum running — Pause to inspect"};
}
