import cpuMark from "../../branding/cpu.svg";

export function ProcessorLogo({ cpu, className = "", wordmark = false }: { cpu?: "6809" | "Z80" | "6502"; className?: string; wordmark?: boolean }) {
  return <span role="img" aria-label={`Eightbit Studio${cpu ? ` ${cpu}` : ""}`} className={`processor-brand ${className}`}>
    <img className="processor-mark" src={cpuMark} alt="" />
    {wordmark && <span className="processor-wordmark">eightbit<span className="processor-edition">STUDIO{cpu ? ` ${cpu}` : ""}</span></span>}
  </span>;
}
