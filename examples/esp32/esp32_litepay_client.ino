/*
 * ESP32 LitePay Client — LNURL-pay + WebSocket real-time notifications
 *
 * This example shows how to:
 *   1. Display LNURL-pay QR code on SSD1306 OLED
 *   2. Receive real-time payment notifications via WebSocket (not polling!)
 *   3. Activate servo motor + LED on payment
 *
 * Hardware:
 *   - ESP32 (any variant)
 *   - SSD1306 OLED 128x64 (I2C: SDA=21, SCL=22)
 *   - Servo motor on GPIO 13
 *   - LED on GPIO 18
 *
 * Dependencies (Arduino Library Manager):
 *   - Adafruit SSD1306
 *   - ArduinoJson
 *   - WebSockets (by Markus Sattler)
 *   - WiFiManager (by tzapu)
 *   - QRCode (by Richard Moore)
 */

#include <WiFi.h>
#include <WiFiManager.h>
#include <WebSocketsClient.h>
#include <ArduinoJson.h>
#include <Wire.h>
#include <Adafruit_SSD1306.h>
#include <ESP32Servo.h>
#include "qrcode.h"

// === Configuration ===
const char* LITEPAY_HOST = "YOUR_LITEPAY_SERVER_IP";
const int   LITEPAY_PORT = 9000;
const char* WALLET_ID    = "YOUR_WALLET_ID";

// === Pins ===
#define SERVO_PIN  13
#define LED_PIN    18
#define SDA_PIN    21
#define SCL_PIN    22

// === Display ===
#define SCREEN_WIDTH  128
#define SCREEN_HEIGHT 64
Adafruit_SSD1306 display(SCREEN_WIDTH, SCREEN_HEIGHT, &Wire, -1);

// === Components ===
Servo servo;
WebSocketsClient webSocket;

void setup() {
  Serial.begin(115200);

  // GPIO setup
  pinMode(LED_PIN, OUTPUT);
  servo.attach(SERVO_PIN);

  // I2C & OLED
  Wire.begin(SDA_PIN, SCL_PIN);
  if (!display.begin(SSD1306_SWITCHCAPVCC, 0x3C)) {
    Serial.println("SSD1306 init failed!");
    while (true);
  }
  display.clearDisplay();
  display.setTextSize(1);
  display.setTextColor(SSD1306_WHITE);
  display.setCursor(0, 0);
  display.println("LitePay Starting...");
  display.display();

  // WiFi (captive portal for first-time setup)
  WiFiManager wifiManager;
  wifiManager.autoConnect("LitePay-Setup");
  Serial.println("WiFi connected: " + WiFi.localIP().toString());

  // Display LNURL QR code
  displayLnurlQR();

  // Connect WebSocket for real-time payment notifications
  // This replaces the 1.5s polling loop used with LNbits!
  String wsPath = "/api/v1/ws/" + String(WALLET_ID);
  webSocket.begin(LITEPAY_HOST, LITEPAY_PORT, wsPath.c_str());
  webSocket.onEvent(webSocketEvent);
  webSocket.setReconnectInterval(5000);

  Serial.println("Ready — waiting for payments via WebSocket");
}

void loop() {
  webSocket.loop();
}

// === WebSocket event handler ===
void webSocketEvent(WStype_t type, uint8_t* payload, size_t length) {
  switch (type) {
    case WStype_CONNECTED:
      Serial.println("WebSocket connected!");
      break;

    case WStype_TEXT: {
      Serial.printf("Payment event: %s\n", payload);

      StaticJsonDocument<1024> doc;
      deserializeJson(doc, payload, length);

      const char* status = doc["payment"]["status"];
      if (strcmp(status, "paid") == 0) {
        int amount = doc["payment"]["amount"];
        Serial.printf("Payment received: %d sats\n", amount);
        onPaymentReceived(amount);
      }
      break;
    }

    case WStype_DISCONNECTED:
      Serial.println("WebSocket disconnected, reconnecting...");
      break;

    default:
      break;
  }
}

// === Payment received — activate outputs ===
void onPaymentReceived(int amountSats) {
  // LED flash
  for (int i = 0; i < 3; i++) {
    digitalWrite(LED_PIN, HIGH);
    delay(200);
    digitalWrite(LED_PIN, LOW);
    delay(200);
  }

  // Servo activation (e.g., vending machine release)
  servo.write(180);
  delay(1000);
  servo.write(0);

  // Show payment on OLED
  display.clearDisplay();
  display.setTextSize(2);
  display.setCursor(10, 10);
  display.printf("%d sats", amountSats);
  display.setTextSize(1);
  display.setCursor(10, 40);
  display.println("Payment received!");
  display.display();

  delay(3000);

  // Return to QR display
  displayLnurlQR();
}

// === Display LNURL QR code on OLED ===
void displayLnurlQR() {
  // In production, fetch the LNURL bech32 from:
  // GET http://LITEPAY_HOST:9000/lnurlp/WALLET_ID
  // For now, use a placeholder
  String lnurl = "LNURL1DP68GURN8GHJ7..."; // Replace with actual LNURL

  QRCode qrcode;
  uint8_t qrcodeData[qrcode_getBufferSize(6)];
  qrcode_initText(&qrcode, qrcodeData, 6, ECC_LOW, lnurl.c_str());

  display.clearDisplay();
  int scale = min(SCREEN_WIDTH / qrcode.size, SCREEN_HEIGHT / qrcode.size);
  int offsetX = (SCREEN_WIDTH - qrcode.size * scale) / 2;
  int offsetY = (SCREEN_HEIGHT - qrcode.size * scale) / 2;

  for (uint8_t y = 0; y < qrcode.size; y++) {
    for (uint8_t x = 0; x < qrcode.size; x++) {
      if (qrcode_getModule(&qrcode, x, y)) {
        display.fillRect(offsetX + x * scale, offsetY + y * scale, scale, scale, SSD1306_WHITE);
      }
    }
  }
  display.display();
}
