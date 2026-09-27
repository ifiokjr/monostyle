type Route = `/${string}`;
type Versioned = `v${number}`;
type Combined = `${Route}/${Versioned}`;

const home: Route = "/home";
const api: Combined = "/api/v2";



export { home, api };
