/// <reference types="vite/client" />

declare module "*.vue" {
  import type { DefineComponent } from "vue";
  // eslint-disable-next-line @typescript-eslint/no-explicit-any -- Vue SFC module shim
  const component: DefineComponent<object, object, any>;
  export default component;
}
