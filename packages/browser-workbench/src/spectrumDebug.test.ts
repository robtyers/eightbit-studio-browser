import {expect,it} from "vitest";
import {spectrumInspection} from "./spectrumDebug";
const revision="a".repeat(64);
const debug={state:"stopped",stopId:1,valuesValid:true,breakpointVerified:true,sourceLine:2,revision,breakpointAddress:24576,registers:{af:0,bc:0,de:0,hl:0,"af'":0,"bc'":0,"de'":0,"hl'":0,ix:0,iy:23610,sp:65534,pc:24576},memory:{startAddress:24576,bytes:Array<number>(64).fill(0)}};
const frame={schemaVersion:1,sessionId:7,firmwareId:"eightbit-spectrum48-basic-0.1-dev",state:"running",debug};
it("binds Spectrum inspection to its worker, source and stopped values",()=>{
 expect(spectrumInspection(frame,7,revision)?.sourceLine).toBe(2);
 expect(spectrumInspection({...frame,debug:null},7,revision)).toBeUndefined();
 expect(spectrumInspection({...frame,state:"failed"},7,revision)?.state).toBe("failed");
 for(const patch of [{revision:"b".repeat(64)},{sourceLine:0},{stopId:-1},{state:"running"},{registers:{pc:0}},{memory:{startAddress:65535,bytes:[0]}}])expect(()=>spectrumInspection({...frame,debug:{...debug,...patch}},7,revision)).toThrow();
 expect(()=>spectrumInspection(frame,8,revision)).toThrow();
 const running=spectrumInspection({...frame,debug:{...debug,state:"running",valuesValid:false,breakpointVerified:false,registers:null,memory:null,sourceLine:null}},7,revision);
 expect(running?.registers).toBeNull();expect(running?.valuesValid).toBe(false);
});
