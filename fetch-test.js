 function fetchTest(data) {
  const x = fetch('https://postman-echo.com', "GET", "", "")
  return x;
}

fetchTest(data);