function Component(options: { selector: string }) {
  return <T extends new (...args: any[]) => any>(target: T): T => target;
}

@Component({ selector: "profile-page" })
class ProfilePage {
  constructor(readonly route: string) {}
}
