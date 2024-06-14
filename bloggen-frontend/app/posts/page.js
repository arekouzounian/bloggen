import fs from 'fs';
import path from 'path';

import Footer from '../components/Footer';

export const revalidate = 60; 

export default function Page() {
    const p = path.resolve(process.cwd(), 'app', 'static'); 
    let posts = fs.readdirSync(p);

    const stats = new Map(); 

    posts = posts.filter((post) => {
        let stat = fs.statSync(path.join(p, post)); 
        if (stat.isDirectory()) {
            stats.set(post, stat);
            return true;
        }
        return false; 
    })

    posts.sort((a, b) => 
        stats.get(a).ctime - stats.get(b).ctime 
    )

    const descriptions = new Map(); 
    posts.forEach((post, _) => {
        let meta_path = path.join(p, post, "meta.json")
        let desc = "no description provided";

        if (fs.existsSync(meta_path)) {
            let meta = fs.readFileSync(meta_path).toString();
            let js = JSON.parse(meta);
            desc = js.Description != null ? js.Description : desc; 
        }

        descriptions.set(post, desc);
    })
    

    return (
        <div className="text-center">
            <h1 className="text-3xl bold m-5"> Blog Posts </h1>
            <div className="grid gap-4 grid-cols-1">
                {posts.map((post, i) => 
                    <a key={i} className="box-content shadow-md border-2 rounded-md p-3 hover:animate-pulse" href={"./posts/" + post}>
                        <div>
                            <p className='bold underline'>{post}</p>
                            <p className="italic opacity-70">{descriptions.get(post)}</p>
                        </div>
                    </a>)}
            </div>
            <Footer />
        </div>
    )
}