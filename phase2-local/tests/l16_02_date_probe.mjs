import assert from "node:assert/strict";

const zone = process.env.TZ;
assert.ok(zone === "Asia/Kolkata" || zone === "America/New_York", `unsupported TZ: ${zone}`);

const cases = zone === "Asia/Kolkata"
  ? [
      { clock: "2024-11-03T18:29:59Z", today: "2024-11-03", clockOffset: -330 },
      { clock: "2024-11-03T18:30:00Z", today: "2024-11-04", clockOffset: -330 },
    ]
  : [
      { clock: "2024-11-03T05:59:59Z", today: "2024-11-03", clockOffset: 240 },
      { clock: "2024-11-03T06:00:00Z", today: "2024-11-03", clockOffset: 300 },
      { clock: "2024-11-04T12:00:00Z", today: "2024-11-04", clockOffset: 300 },
    ];

console.log(`runtime=${process.version} TZ=${zone} tzdata=${process.versions.tz}`);

for (const { clock: clockISO, today, clockOffset } of cases) {
  const clock = new Date(clockISO);
  const localToday = [clock.getFullYear(), String(clock.getMonth() + 1).padStart(2, "0"), String(clock.getDate()).padStart(2, "0")].join("-");
  assert.equal(localToday, today, `${clockISO} local today`);
  assert.equal(clock.getTimezoneOffset(), clockOffset, `${clockISO} clock offset`);

  const dateOnly = localToday;
  const shifted = new Date(dateOnly);
  const parsedOffset = shifted.getTimezoneOffset();
  shifted.setDate(shifted.getDate() - 30);
  const shiftedOffset = shifted.getTimezoneOffset();
  const shiftedISO = shifted.toISOString();

  const expected = zone === "Asia/Kolkata"
    ? { parsedOffset: -330, shiftedOffset: -330, shiftedISO: today === "2024-11-03" ? "2024-10-04T00:00:00.000Z" : "2024-10-05T00:00:00.000Z" }
    : { parsedOffset: today === "2024-11-03" ? 240 : 300, shiftedOffset: 240, shiftedISO: today === "2024-11-03" ? "2024-10-04T00:00:00.000Z" : "2024-10-04T23:00:00.000Z" };

  assert.equal(parsedOffset, expected.parsedOffset, `${clockISO} parsed anchor offset`);
  assert.equal(shiftedOffset, expected.shiftedOffset, `${clockISO} shifted offset`);
  assert.equal(shiftedISO, expected.shiftedISO, `${clockISO} shifted full ISO`);

  if (zone === "America/New_York" && today === "2024-11-04") {
    const naiveUtcAnchor = new Date(dateOnly);
    naiveUtcAnchor.setUTCDate(naiveUtcAnchor.getUTCDate() - 30);
    const naiveUtcCutoff = naiveUtcAnchor.toISOString();
    const naiveUtcDate = naiveUtcCutoff.slice(0, 10);
    const actualDate = shiftedISO.slice(0, 10);
    assert.equal(naiveUtcDate, "2024-10-05", "UTC-midnight anchor minus 30 UTC days");
    assert.notEqual(naiveUtcDate, actualDate, "naive UTC cutoff date differs from local calendar shift date");
    console.log(`${clockISO}: today=${localToday} clockOffset=${clockOffset} parsedOffset=${parsedOffset} shiftedOffset=${shiftedOffset} shifted=${shiftedISO} naiveUTCCutoff=${naiveUtcCutoff} naiveUTCDate=${naiveUtcDate} actualDate=${actualDate}`);
  } else {
    console.log(`${clockISO}: today=${localToday} clockOffset=${clockOffset} parsedOffset=${parsedOffset} shiftedOffset=${shiftedOffset} shifted=${shiftedISO}`);
  }
}
