// Jednoduché počítadlo aktivity projektu (koľkokrát si spustil program) – bez XP a odznakov.
export function createActivity({ getSettings, saveSettings }) {
  const data = () => getSettings().activity || {};
  let busy = Promise.resolve();
  return {
    runs: (dir) => data()[dir]?.runs || 0,
    run(dir) {
      if (!dir) return busy;
      busy = busy.then(() => {
        const all = { ...data() };
        all[dir] = { ...all[dir], runs: (all[dir]?.runs || 0) + 1 };
        return saveSettings({ activity: all });
      });
      return busy;
    },
  };
}
