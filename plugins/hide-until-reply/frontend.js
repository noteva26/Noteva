/**
 * 回复可见插件
 * 隐藏部分内容，用户回复后才能查看
 */
(function() {
  const PLUGIN_ID = 'hide-until-reply';
  let isProcessing = false;
  
  // 等待 Noteva SDK 加载
  function waitForNoteva(callback) {
    if (typeof Noteva !== 'undefined') {
      callback();
    } else {
      setTimeout(() => waitForNoteva(callback), 100);
    }
  }
  
  waitForNoteva(function() {
    // 检查用户是否已回复该文章
    async function checkUnlocked(articleId) {
      // 先确保用户状态已检查
      const currentUser = await Noteva.user.check();
      
      if (!currentUser) {
        return false;
      }
      
      try {
        // 获取文章评论，检查当前用户是否有评论
        const comments = await Noteva.api.get(`/comments/${articleId}`);
        
        if (!comments.comments) return false;
        
        // 递归检查评论（包括回复）
        function hasUserComment(commentList) {
          for (const c of commentList) {
            if (c.user_id === currentUser.id) return true;
            if (c.replies && hasUserComment(c.replies)) return true;
          }
          return false;
        }
        
        return hasUserComment(comments.comments);
      } catch (e) {
        console.log('[hide-until-reply] Check error:', e);
        return false;
      }
    }
    
    // 解锁隐藏内容
    function unlockContent(articleId) {
      const elements = document.querySelectorAll(`.noteva-hidden-content[data-article-id="${articleId}"]`);
      elements.forEach(el => {
        const template = el.querySelector('.noteva-hidden-template');
        
        if (template) {
          const revealed = document.createElement('div');
          revealed.className = 'noteva-revealed-content';
          revealed.innerHTML = template.innerHTML;
          el.parentNode.replaceChild(revealed, el);
        }
      });
    }

    // 初始化：查找页面上的隐藏内容并检查是否需要解锁
    async function initHiddenContent() {
      if (isProcessing) return;
      
      const hiddenElements = document.querySelectorAll('.noteva-hidden-content[data-article-id]');
      if (hiddenElements.length === 0) return;
      
      isProcessing = true;
      console.log('[hide-until-reply] Found', hiddenElements.length, 'hidden elements');
      
      for (const el of hiddenElements) {
        const articleId = parseInt(el.dataset.articleId, 10);
        if (articleId) {
          const unlocked = await checkUnlocked(articleId);
          console.log('[hide-until-reply] Article', articleId, 'unlocked:', unlocked);
          if (unlocked) {
            unlockContent(articleId);
          }
        }
      }
      
      isProcessing = false;
    }
    
    // 监听评论创建事件
    Noteva.hooks.on('comment_after_create', async (data) => {
      const articleId = data.articleId;
      if (articleId) {
        setTimeout(() => {
          unlockContent(articleId);
          Noteva.ui.toast('内容已解锁！', 'success');
        }, 500);
      }
    });
    
    // 监听内容渲染事件
    Noteva.hooks.on('content_render', () => {
      setTimeout(initHiddenContent, 100);
    });
    
    // 使用 MutationObserver 监听 DOM 变化
    function startObserver() {
      if (!document.body) {
        // body 还没准备好，等一下再试
        setTimeout(startObserver, 100);
        return;
      }
      
      const observer = new MutationObserver((mutations) => {
        for (const mutation of mutations) {
          if (mutation.type === 'childList') {
            // 检查是否有新的隐藏内容元素
            const hasHiddenContent = document.querySelector('.noteva-hidden-content[data-article-id]');
            if (hasHiddenContent) {
              setTimeout(initHiddenContent, 100);
              break;
            }
          }
        }
      });
      
      // 开始监听
      observer.observe(document.body, {
        childList: true,
        subtree: true
      });
    }
    
    startObserver();
    
    // 页面加载时初始化
    Noteva.ready(() => {
      // 延迟执行，等待 React 渲染完成
      setTimeout(initHiddenContent, 500);
      // 再次检查，以防第一次太早
      setTimeout(initHiddenContent, 1500);
    });
  });
})();
