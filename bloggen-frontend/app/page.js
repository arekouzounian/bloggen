import fs from 'fs';
import path from 'path';

import Footer from './components/Footer';

export const revalidate = 60;

export default function Page() {
  const p = path.resolve(process.cwd(), 'app', 'static');
  let posts = fs.readdirSync(p);

  posts = posts.filter((post) =>
    fs.statSync(path.join(p, post)).isDirectory()
  );

  const js = new Map();

  posts.forEach((post, _) => {
    let meta_path = path.join(p, post, "meta.json")

    if (fs.existsSync(meta_path)) {
      let meta = fs.readFileSync(meta_path).toString();
      js.set(post, JSON.parse(meta));
    }
  });

  posts.sort((a, b) =>
    js.get(b).LastChanged - js.get(a).LastChanged
  )

  return (
    <div className="text-center">
      <h1 className="text-3xl bold m-5"> Blog Posts </h1>
      <div className="grid gap-4 grid-cols-1">
        {posts.map((post, i) =>
          <a key={i} className="box-content shadow-md border-2 rounded-md p-3 hover:animate-pulse" href={post}>
            <div>
              <p className='bold underline'>{js.get(post).Title}</p>
              <p className='bold italic opacity-50 text-xs'>{new Date(js.get(post).LastChanged * 1000).toLocaleDateString()}</p>
              <p className="italic opacity-70">{js.get(post).Description}</p>
            </div>
          </a>)}
      </div>
      <Footer />
    </div>
  )
}
