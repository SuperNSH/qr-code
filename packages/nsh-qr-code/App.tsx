import { mount, signal, Text, View } from "nsh";
import { type ScanState, useQrScanner } from "./src";

const message = signal("nothing yet");

function describe(state: ScanState): string {
  if (state.status === "failed") return `scan failed ${state.error.code}`;
  return `scan ${state.status}`;
}

function App() {
  const scanner = useQrScanner({
    onCodeScanned: (code) => message.set(`scanned ${code.text}`),
  });
  return (
    <View class="flex-1 gap-3 p-4">
      <Text testID="title" class="text-foreground">nsh-qr-code</Text>
      <Text testID="state" class="text-foreground">{describe(scanner.state())}</Text>
      <Text testID="message" class="text-foreground">{message()}</Text>
      <Text testID="scan" role="button" onPress={() => scanner.start()} class="rounded-(--radius) bg-muted px-3 py-2">
        Scan
      </Text>
      <Text testID="stop" role="button" onPress={() => scanner.stop()} class="rounded-(--radius) bg-muted px-3 py-2">
        Stop
      </Text>
    </View>
  );
}

mount(App);
