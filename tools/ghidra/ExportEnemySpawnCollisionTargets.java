// Ghidra headless helper: export focused enemy spawn/collision targets.
//@category OPPW4

import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileOptions;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.symbol.Reference;

import java.io.File;
import java.io.FileWriter;
import java.io.PrintWriter;

public class ExportEnemySpawnCollisionTargets extends GhidraScript {
    private static final String OUT =
        "C:/Users/Osef/Documents/Codex/oppw4-sdk-split/oppw4-sdk/docs/reverse-notes/enemy-spawn-collision-targets-2026-06-02.txt";

    private static final String[] FUNCTIONS = {
        "141230780",
        "141230830",
        "1412308e0",
        "141230b20",
        "141235a90",
        "1412505b0",
        "141250830",
        "141254340",
        "1412547e0",
        "141254a70",
        "14124e670",
        "1415d1320",
        "141231100",
    };

    @Override
    protected void run() throws Exception {
        File out = new File(OUT);
        out.getParentFile().mkdirs();
        try (PrintWriter pw = new PrintWriter(new FileWriter(out))) {
            pw.println("OPPW4.exe enemy spawn/collision targets");
            pw.println("image_base=" + currentProgram.getImageBase());
            pw.println();

            DecompInterface ifc = new DecompInterface();
            ifc.setOptions(new DecompileOptions());
            ifc.openProgram(currentProgram);

            for (String address : FUNCTIONS) {
                dumpFunction(pw, ifc, address);
                dumpCallers(pw, address);
                dumpWindow(pw, address, 6, 16);
            }

            ifc.dispose();
        }
        println("Wrote " + out.getAbsolutePath());
    }

    private void dumpFunction(PrintWriter pw, DecompInterface ifc, String address) throws Exception {
        Function fn = currentProgram.getListing().getFunctionContaining(toAddr(address));
        if (fn == null) {
            pw.println(address + " -> <no function>");
            return;
        }

        pw.println();
        pw.println("============================================================");
        pw.println("FUNCTION " + fn.getName() + " @ " + fn.getEntryPoint());
        pw.println("BODY " + fn.getBody());
        pw.println("CALLEES:");
        printCallees(pw, fn);
        pw.println("DECOMPILE:");

        DecompileResults result = ifc.decompileFunction(fn, 120, monitor);
        if (result.decompileCompleted()) {
            pw.println(result.getDecompiledFunction().getC());
        } else {
            pw.println("<decompile failed: " + result.getErrorMessage() + ">");
        }
    }

    private void dumpCallers(PrintWriter pw, String address) throws Exception {
        Address entry = toAddr(address);
        pw.println("CALLERS:");
        Reference[] refs = getReferencesTo(entry);
        for (Reference ref : refs) {
            if (!ref.getReferenceType().isCall()) {
                continue;
            }
            Function caller = currentProgram.getListing().getFunctionContaining(ref.getFromAddress());
            String name = caller == null ? "<unknown>" : caller.getName();
            pw.println("  " + ref.getFromAddress() + " from " + name);
        }
    }

    private void dumpWindow(PrintWriter pw, String address, int before, int after) throws Exception {
        Address center = toAddr(address);
        pw.println("WINDOW around " + center);
        var listing = currentProgram.getListing();
        Instruction inst = listing.getInstructionContaining(center);
        if (inst == null) {
            pw.println("<no instruction>");
            return;
        }
        for (int i = 0; i < before; i++) {
            Instruction prev = listing.getInstructionBefore(inst.getAddress());
            if (prev == null) {
                break;
            }
            inst = prev;
        }
        for (int i = 0; i < before + after + 1 && inst != null; i++) {
            byte[] bytes = inst.getBytes();
            pw.printf("%s  %-32s  %s%n", inst.getAddress(), formatBytes(bytes), inst);
            inst = listing.getInstructionAfter(inst.getAddress());
        }
    }

    private void printCallees(PrintWriter pw, Function fn) {
        var listing = currentProgram.getListing();
        var instructions = listing.getInstructions(fn.getBody(), true);
        while (instructions.hasNext() && !monitor.isCancelled()) {
            Instruction inst = instructions.next();
            for (Reference ref : inst.getReferencesFrom()) {
                if (!ref.getReferenceType().isCall()) {
                    continue;
                }
                Function callee = listing.getFunctionAt(ref.getToAddress());
                String name = callee == null ? "<external/unknown>" : callee.getName();
                pw.println("  " + inst.getAddress() + " -> " + ref.getToAddress() + " " + name);
            }
        }
    }

    private String formatBytes(byte[] bytes) {
        StringBuilder builder = new StringBuilder();
        for (byte value : bytes) {
            if (builder.length() > 0) {
                builder.append(' ');
            }
            builder.append(String.format("%02x", value & 0xff));
        }
        return builder.toString();
    }
}
