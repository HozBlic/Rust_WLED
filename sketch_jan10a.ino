#include <FastLED.h>

#define NUM_LEDS 138
#define DATA_PIN 2

String serialReceive;
unsigned long darkTimeout = 2000;
unsigned long lastUpdate = 0;
int charsToMove = 6 * 14;

CRGB leds[NUM_LEDS];
CRGB targetColors[NUM_LEDS]; // Target colors for transition

void setup() {
  FastLED.addLeds<NEOPIXEL, DATA_PIN>(leds, NUM_LEDS);
  FastLED.setBrightness(50);
  Serial.setRxBufferSize(NUM_LEDS * 6 + 1);
  Serial.begin(1000000);
  Serial.setTimeout(10);
}

void loop() {
  // Check for incoming serial data
  if (Serial.available()) {
    serialReceive = Serial.readString(); // Read the incoming data as a string
    serialReceive.trim();

    // Pad the data if shorter
    while (serialReceive.length() < (NUM_LEDS - 8 * 6) * 6) {
      serialReceive += "0";
    }

    // Crop if longer
    if (serialReceive.length() > NUM_LEDS * 6) {
      serialReceive = serialReceive.substring(0, NUM_LEDS * 6);
    }

    // Move the first few LEDs to the end
    String firstTen = serialReceive.substring(0, charsToMove);
    String restOfString = serialReceive.substring(charsToMove);
    serialReceive = restOfString + firstTen;

    // Add padding in front (8 LEDs)
    while (serialReceive.length() < NUM_LEDS * 6) {
      serialReceive = "0" + serialReceive;
    }

    // Update targetColors[] array with new colors
    for (int i = 0; i < NUM_LEDS; i++) {
      targetColors[i] = strtol(("0x" + serialReceive.substring(0, 6)).c_str(), NULL, 16);
      serialReceive.remove(0, 6);
    }

    lastUpdate = millis();
  }

  // Smoothly transition LEDs to targetColors[]
  bool needsUpdate = false;
  for (int i = 0; i < NUM_LEDS; i++) {
    if (leds[i] != targetColors[i]) {
      leds[i] = blend(leds[i], targetColors[i], 16); // Blend with a fixed speed
      needsUpdate = true; // Mark as needing an update
    }
  }

  // Update LEDs only if there were changes
  if (needsUpdate) {
    FastLED.show();
  }

  // Turn off LEDs after darkTimeout
  if (millis() - lastUpdate >= darkTimeout) {
    FastLED.clear(true); // Immediately clear all LEDs
  }

  delay(20); // Add a small delay to stabilize transitions
}
