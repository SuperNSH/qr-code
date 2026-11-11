import { mount, signal, Text, View } from "nsh";
import { fakeFrameSource, memoryBackend, QrScanner, useQrScanner } from "../../src";
import { codeFrame } from "../helpers";

const codes = signal<string[]>([]);
const backend = memoryBackend();

function App() {
  const scanner = useQrScanner({
    scanner: new QrScanner({ backend }),
    onCodeScanned: (code) => codes.set([...codes(), code.text]),
  });
  const start = async () => {
    // One frame, one code: the scan ends after reporting it, and the text settles on it.
    backend.setFrameSource(fakeFrameSource([await codeFrame("first-code")]));
    scanner.start();
  };
  return (
    <View class="flex-1 items-center justify-center gap-3 p-4">
      <Text testID="codes" class="text-foreground">{codes().join(",")}</Text>
      <Text
        testID="start"
        role="button"
        onPress={start}
        class="rounded-(--radius) bg-muted px-3 py-2"
      >
        Start
      </Text>
    </View>
  );
}

mount(App);
